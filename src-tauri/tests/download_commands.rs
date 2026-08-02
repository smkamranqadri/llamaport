//! The command layer's decisions, taken apart from the app that hosts them.
//!
//! Everything here is the part of a download that happens before a byte moves or after
//! the last one has: what may start, what a URL will be saved as, what a cancel means,
//! and how a finished transfer is described.
//!
//! The manager is driven over a stand-in engine rather than a real transfer. What is
//! under test is the bookkeeping around one — what a job settles as, what the screen is
//! told, what happens once a file lands — and a real transfer would only add a server.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use llama_cpp_hub_lib::download::{
    Control, Phase, Progress, ProgressSink, Spec, CANCELLED, DEFAULT_FLUSH_EVERY,
    DEFAULT_PROGRESS_EVERY,
};
use llama_cpp_hub_lib::downloads::{
    admit, cancellable, file_name_for, settle, spec_for, DownloadJob, DownloadState, Downloads,
    Engine, Options,
};
use llama_cpp_hub_lib::runner::EventSink;

fn scratch(name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let unique = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "llama-hub-dlcmd-{}-{unique}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

const URL: &str = "https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF/resolve/main/Qwen3.6-35B-A3B-UD-Q3_K_XL.gguf";
const FILE: &str = "Qwen3.6-35B-A3B-UD-Q3_K_XL.gguf";

#[derive(Default)]
struct Recorder {
    events: Mutex<Vec<(String, serde_json::Value)>>,
}

impl EventSink for Recorder {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        self.events
            .lock()
            .expect("events lock")
            .push((event.into(), payload));
    }
}

impl Recorder {
    fn names(&self) -> Vec<String> {
        self.events
            .lock()
            .expect("events lock")
            .iter()
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn payloads(&self, event: &str) -> Vec<serde_json::Value> {
        self.events
            .lock()
            .expect("events lock")
            .iter()
            .filter(|(name, _)| name == event)
            .map(|(_, payload)| payload.clone())
            .collect()
    }
}

struct Manager {
    downloads: Downloads,
    events: Arc<Recorder>,
    landings: Arc<AtomicU32>,
}

fn manager(engine: Engine) -> Manager {
    let events = Arc::new(Recorder::default());
    let landings = Arc::new(AtomicU32::new(0));
    let counted = Arc::clone(&landings);
    let downloads = Downloads::with_engine(
        events.clone(),
        Arc::new(move || {
            counted.fetch_add(1, Ordering::Relaxed);
        }),
        engine,
    );
    Manager {
        downloads,
        events,
        landings,
    }
}

/// Stands in for a transfer that is still running: it ends when, and only when, it is
/// told to.
fn wait_for_cancel(control: &Control) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !control.cancelled() {
        assert!(
            Instant::now() < deadline,
            "the cancel never reached the transfer"
        );
        thread::sleep(Duration::from_millis(5));
    }
    Err(CANCELLED.to_string())
}

fn eventually(what: &str, ready: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !ready() {
        assert!(Instant::now() < deadline, "{what}");
        thread::sleep(Duration::from_millis(5));
    }
}

fn settled(downloads: &Downloads, id: &str) -> DownloadJob {
    let held = || {
        downloads
            .snapshot()
            .into_iter()
            .find(|job| job.id == id)
            .expect("the job must stay tracked until it is cleared")
    };
    eventually(&format!("{id} never settled"), || {
        held().state != DownloadState::Active
    });
    held()
}

/// The engine is lent a `&Spec` for the length of the call, so a test that wants to look
/// at what it was handed takes a copy of it.
struct Handed {
    url: String,
    dest: PathBuf,
    segments: usize,
    stall_after: Duration,
    retry_backoff: Duration,
    rate_limit: Option<u64>,
    verify: bool,
    progress_every: Duration,
    flush_every: Duration,
}

fn handed(spec: &Spec) -> Handed {
    Handed {
        url: spec.url.clone(),
        dest: spec.dest.clone(),
        segments: spec.segments,
        stall_after: spec.stall_after,
        retry_backoff: spec.retry_backoff,
        rate_limit: spec.rate_limit,
        verify: spec.verify,
        progress_every: spec.progress_every,
        flush_every: spec.flush_every,
    }
}

fn job(id: &str, url: &str, file_name: &str, state: DownloadState) -> DownloadJob {
    let mut job = DownloadJob::begin(id, url, &PathBuf::from("/models").join(file_name));
    job.state = state;
    job
}

#[test]
fn a_file_url_yields_the_name_it_will_be_saved_as() {
    assert_eq!(file_name_for(URL).as_deref(), Ok(FILE));

    assert_eq!(
        file_name_for(&format!("{URL}?download=true")).as_deref(),
        Ok(FILE),
        "the URL the download button copies carries a query"
    );

    assert_eq!(
        file_name_for(
            "https://hf.co/repo/owner/resolve/main/nested/dir/Model-Q4_K_M.gguf#fragment"
        )
        .as_deref(),
        Ok("Model-Q4_K_M.gguf")
    );
}

#[test]
fn anything_that_is_not_a_hugging_face_gguf_is_refused() {
    let refused = [
        "http://huggingface.co/repo/resolve/main/model.gguf",
        "https://example.com/repo/resolve/main/model.gguf",
        "https://huggingface.co.evil.example/repo/resolve/main/model.gguf",
        "https://huggingface.co@evil.example/repo/resolve/main/model.gguf",
        "https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF",
        "https://huggingface.co/repo/resolve/main/README.md",
        "https://huggingface.co/repo/resolve/main/",
        "",
    ];

    for url in refused {
        assert!(file_name_for(url).is_err(), "{url} should be refused");
    }
}

#[test]
fn a_file_already_in_the_models_directory_is_not_downloaded_again() {
    let dir = scratch("existing");
    fs::write(dir.join(FILE), b"already here").expect("seed");

    let error = admit(URL, &dir, &[]).expect_err("refused");
    assert!(error.contains("already in the models directory"), "{error}");
}

#[test]
fn a_transfer_that_has_not_settled_holds_the_line() {
    let dir = scratch("one-at-a-time");

    let same =
        admit(URL, &dir, &[job("dl-1", URL, FILE, DownloadState::Active)]).expect_err("refused");
    assert!(same.contains("already downloading"), "{same}");

    let renamed = admit(
        URL,
        &dir,
        &[job(
            "dl-1",
            "https://huggingface.co/other/repo/resolve/main/Qwen3.6-35B-A3B-UD-Q3_K_XL.gguf",
            FILE,
            DownloadState::Active,
        )],
    )
    .expect_err("refused");
    assert!(renamed.contains("from another URL"), "{renamed}");

    let other = admit(
        URL,
        &dir,
        &[job(
            "dl-1",
            "https://huggingface.co/other/repo/resolve/main/Other-Q4_K_M.gguf",
            "Other-Q4_K_M.gguf",
            DownloadState::Active,
        )],
    )
    .expect_err("refused");
    assert!(other.contains("one file at a time"), "{other}");
}

#[test]
fn a_settled_transfer_does_not_hold_the_line() {
    let dir = scratch("settled");
    let history = [
        job("dl-1", URL, FILE, DownloadState::Cancelled),
        job(
            "dl-2",
            "https://huggingface.co/a/b/resolve/main/x.gguf",
            "x.gguf",
            DownloadState::Failed,
        ),
        job(
            "dl-3",
            "https://huggingface.co/a/b/resolve/main/y.gguf",
            "y.gguf",
            DownloadState::Complete,
        ),
    ];

    assert_eq!(admit(URL, &dir, &history), Ok(dir.join(FILE)));
}

#[test]
fn cancelling_a_download_that_is_no_longer_running_is_not_an_error() {
    let jobs = [
        job("dl-1", URL, FILE, DownloadState::Complete),
        job("dl-2", URL, FILE, DownloadState::Active),
    ];

    assert_eq!(cancellable(&jobs, "dl-2"), Ok(true));
    assert_eq!(
        cancellable(&jobs, "dl-1"),
        Ok(false),
        "a transfer that finished first has nothing to signal"
    );
    assert!(
        cancellable(&jobs, "dl-9").is_err(),
        "an id that was never started is a different mistake"
    );
}

#[test]
fn a_cancelled_transfer_is_not_reported_as_a_failure() {
    assert_eq!(settle(Ok(())), (DownloadState::Complete, None));
    assert_eq!(
        settle(Err(CANCELLED.to_string())),
        (DownloadState::Cancelled, None)
    );
    assert_eq!(
        settle(Err("HTTP 404".to_string())),
        (DownloadState::Failed, Some("HTTP 404".to_string()))
    );
}

#[test]
fn spec_for_turns_the_stored_options_into_a_spec() {
    let options = Options {
        segments: 8,
        rate_limit: Some(10_000_000),
        verify: false,
    };
    let spec = spec_for(URL, &PathBuf::from("/models").join(FILE), &options);

    assert_eq!(spec.url, URL);
    assert_eq!(spec.dest, PathBuf::from("/models").join(FILE));
    assert_eq!(spec.segments, 8);
    assert_eq!(spec.rate_limit, Some(10_000_000));
    assert!(!spec.verify);

    let unlimited = spec_for(
        URL,
        &PathBuf::from(FILE),
        &Options {
            rate_limit: Some(0),
            ..Options::default()
        },
    );
    assert_eq!(
        unlimited.rate_limit, None,
        "a limit of zero is no limit, not a stalled transfer"
    );
}

/// The engine charges a 64 KB buffer at a time and sleeps out whatever the budget does not
/// cover, so a limit of a byte a second is an eighteen hour sleep between reads: the
/// transfer stops responding to a cancel and the app refuses every download behind it.
#[test]
fn a_rate_limit_too_low_to_transfer_at_is_raised_to_one_that_is() {
    for asked in [1, 100, 8_000] {
        let spec = spec_for(
            URL,
            &PathBuf::from(FILE),
            &Options {
                rate_limit: Some(asked),
                ..Options::default()
            },
        );
        let rate = spec.rate_limit.expect("a limit was asked for");
        assert!(
            rate >= 64 * 1024,
            "{asked} bytes per second reached the engine as {rate}, which parks a buffer \
             for longer than a second"
        );
    }
}

#[test]
fn a_refused_request_starts_nothing_and_reports_nothing() {
    let dir = scratch("refused");
    let events = Arc::new(Recorder::default());
    let downloads = Downloads::new(events.clone(), Arc::new(|| {}));

    let error = downloads
        .start("https://example.com/model.gguf", &dir, &Options::default())
        .expect_err("refused");
    assert!(!error.is_empty());

    assert!(downloads.snapshot().is_empty());
    assert!(events.names().is_empty());
}

/// Not the builder in isolation: what is under test is that the options the user stored
/// survive the trip from `start` into the call the engine is made with.
#[test]
fn the_stored_options_reach_the_engine() {
    let dir = scratch("options");
    let seen = Arc::new(Mutex::new(None));
    let captured = Arc::clone(&seen);

    let engine: Engine = Arc::new(move |spec: &Spec, _: &Control, _: &dyn ProgressSink| {
        *captured.lock().expect("spec lock") = Some(handed(spec));
        Ok(())
    });
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(
            URL,
            &dir,
            &Options {
                segments: 8,
                rate_limit: Some(10_000_000),
                verify: false,
            },
        )
        .expect("admitted");
    settled(&manager.downloads, &started[0].id);

    let spec = seen
        .lock()
        .expect("spec lock")
        .take()
        .expect("the engine has to be handed a spec");

    assert_eq!(spec.url, URL);
    assert_eq!(spec.dest, dir.join(FILE));
    assert_eq!(spec.segments, 8);
    assert_eq!(spec.rate_limit, Some(10_000_000));
    assert!(!spec.verify);

    // The engine's bounds are the manager's to supply, and none of them is optional: a
    // zero stall timeout is a hung segment nothing reissues, and a zero reporting floor
    // is a report per read — millions of them on a 21 GB file.
    assert!(
        spec.stall_after >= Duration::from_secs(5),
        "a segment needs a silence it is judged against, got {:?}",
        spec.stall_after
    );
    assert!(!spec.retry_backoff.is_zero(), "a retry has to wait");
    assert_eq!(spec.progress_every, DEFAULT_PROGRESS_EVERY);
    assert_eq!(spec.flush_every, DEFAULT_FLUSH_EVERY);
}

/// The rule the manager exists to enforce, taken against a transfer that is actually
/// running rather than against a list handed to `admit` by hand.
#[test]
fn a_second_transfer_is_refused_while_the_first_is_still_running() {
    let dir = scratch("live-line");
    let engines = Arc::new(AtomicU32::new(0));
    let counted = Arc::clone(&engines);

    let engine: Engine = Arc::new(move |_: &Spec, control: &Control, _: &dyn ProgressSink| {
        counted.fetch_add(1, Ordering::Relaxed);
        wait_for_cancel(control)
    });
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let running = started[0].id.clone();
    eventually("the transfer never reached the engine", || {
        engines.load(Ordering::Relaxed) == 1
    });

    let refused = manager
        .downloads
        .start(
            "https://huggingface.co/a/b/resolve/main/Other-Q4_K_M.gguf",
            &dir,
            &Options::default(),
        )
        .expect_err("one file at a time");
    assert!(refused.contains("one file at a time"), "{refused}");
    assert_eq!(
        manager.downloads.snapshot().len(),
        1,
        "a refused request must not be tracked"
    );
    assert_eq!(
        engines.load(Ordering::Relaxed),
        1,
        "a refused request must not reach the engine"
    );

    manager.downloads.cancel(&running).expect("cancelled");
    assert_eq!(
        settled(&manager.downloads, &running).state,
        DownloadState::Cancelled
    );
}

/// On a fresh install the configured models directory does not exist yet, and the engine
/// opens its `.part` inside it.
#[test]
fn the_models_directory_is_made_for_an_admitted_transfer_and_not_for_a_refused_one() {
    let dir = scratch("missing-dir").join("models");
    let engine: Engine = Arc::new(|_: &Spec, _: &Control, _: &dyn ProgressSink| Ok(()));
    let manager = manager(engine);

    let refused = manager
        .downloads
        .start("https://example.com/model.gguf", &dir, &Options::default())
        .expect_err("refused");
    assert!(!refused.is_empty());
    assert!(
        !dir.exists(),
        "a URL that was turned away must leave nothing behind"
    );

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    assert!(dir.is_dir(), "the transfer has nowhere to write");
    assert_eq!(
        settled(&manager.downloads, &started[0].id).state,
        DownloadState::Complete
    );
}

#[test]
fn a_started_transfer_is_reported_as_it_runs_and_lands_when_it_finishes() {
    let dir = scratch("start");
    let seen = Arc::new(Mutex::new(None));
    let captured = Arc::clone(&seen);

    let engine: Engine = Arc::new(move |spec: &Spec, _: &Control, sink: &dyn ProgressSink| {
        *captured.lock().expect("spec lock") = Some(spec.dest.clone());
        sink.report(Progress {
            phase: Phase::Transferring,
            completed: 40_000,
            total: Some(64_000),
            bytes_per_second: Some(20_000.0),
        });
        Ok(())
    });
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].state, DownloadState::Active);
    let id = started[0].id.clone();

    let job = settled(&manager.downloads, &id);
    assert_eq!(job.state, DownloadState::Complete);
    assert_eq!(job.error, None);
    assert!(job.finished_secs.is_some());
    assert_eq!(
        *seen.lock().expect("spec lock"),
        Some(dir.join(FILE)),
        "the transfer must be pointed at the models directory"
    );

    // The report has to reach the job itself, not only the event bus: a screen opened
    // mid-transfer reads the snapshot rather than the events it was not there for.
    assert_eq!(job.phase, Some(Phase::Transferring));
    assert_eq!(job.total, Some(64_000));
    assert_eq!(
        job.completed, 64_000,
        "a finished transfer ends on the whole file rather than on its last sample"
    );
    assert_eq!(job.bytes_per_second, None, "a settled job is not moving");

    eventually("the settled job was never announced", || {
        manager.events.names().len() == 3
    });
    let names = manager.events.names();
    assert_eq!(
        names,
        ["download:state", "download:progress", "download:state"],
        "admission, the report, and the outcome each have to reach the screen"
    );

    let progress = manager.events.payloads("download:progress");
    assert_eq!(progress[0]["id"].as_str(), Some(id.as_str()));
    assert_eq!(progress[0]["phase"].as_str(), Some("transferring"));
    assert_eq!(progress[0]["completed"].as_u64(), Some(40_000));
    assert_eq!(progress[0]["total"].as_u64(), Some(64_000));

    assert_eq!(
        manager.landings.load(Ordering::Relaxed),
        1,
        "a file in the models directory has to be taken into the catalog exactly once"
    );
}

#[test]
fn a_transfer_that_fails_takes_nothing_into_the_catalog() {
    let dir = scratch("failed");
    let engine: Engine =
        Arc::new(|_: &Spec, _: &Control, _: &dyn ProgressSink| Err("HTTP 404".to_string()));
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let job = settled(&manager.downloads, &started[0].id);

    assert_eq!(job.state, DownloadState::Failed);
    assert_eq!(job.error.as_deref(), Some("HTTP 404"));

    // The outcome is announced after the catalog has been given its chance at the file, so
    // by the time the event lands a wrongly taken-in file would already be counted.
    eventually("the settled job was never announced", || {
        manager.events.payloads("download:state").len() == 2
    });
    assert_eq!(
        manager.landings.load(Ordering::Relaxed),
        0,
        "nothing landed, so the catalog has nothing to take in"
    );
}

#[test]
fn cancelling_reaches_the_transfer_and_settles_the_job() {
    let dir = scratch("cancel");
    let engine: Engine =
        Arc::new(|_: &Spec, control: &Control, _: &dyn ProgressSink| wait_for_cancel(control));
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let id = started[0].id.clone();

    let unknown = manager
        .downloads
        .cancel("dl-9")
        .expect_err("an id that was never started is a mistake, not a no-op");
    assert!(unknown.contains("no download dl-9"), "{unknown}");

    manager.downloads.cancel(&id).expect("cancelled");

    let job = settled(&manager.downloads, &id);
    assert_eq!(job.state, DownloadState::Cancelled);
    assert_eq!(job.error, None, "a cancellation is not a failure");

    eventually("the settled job was never announced", || {
        manager.events.payloads("download:state").len() == 2
    });
    assert_eq!(manager.landings.load(Ordering::Relaxed), 0);

    let again = manager
        .downloads
        .cancel(&id)
        .expect("cancelling a transfer that has already stopped is nothing to do");
    assert_eq!(again.len(), 1);
    assert_eq!(
        manager.downloads.snapshot()[0].state,
        DownloadState::Cancelled
    );
}

#[test]
fn clearing_drops_what_has_settled_and_leaves_what_is_running() {
    let dir = scratch("clear");
    let other = "https://huggingface.co/a/b/resolve/main/Other-Q4_K_M.gguf";

    let engine: Engine = Arc::new(|spec: &Spec, control: &Control, _: &dyn ProgressSink| {
        if spec.url.ends_with("Other-Q4_K_M.gguf") {
            return Err("HTTP 500".to_string());
        }
        wait_for_cancel(control)
    });
    let manager = manager(engine);

    let first = manager
        .downloads
        .start(other, &dir, &Options::default())
        .expect("admitted");
    let done = first[0].id.clone();
    assert_eq!(
        settled(&manager.downloads, &done).state,
        DownloadState::Failed
    );

    let second = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let running = second.last().expect("the job just started").id.clone();
    eventually("the settled job was never announced", || {
        manager.events.payloads("download:state").len() == 3
    });

    let remaining = manager.downloads.clear();
    let ids: Vec<&str> = remaining.iter().map(|job| job.id.as_str()).collect();
    assert_eq!(
        ids,
        [running.as_str()],
        "clear must answer with what is left, not with what was there"
    );
    assert_eq!(manager.downloads.snapshot().len(), 1);

    // Answering the caller is not enough: clear is the only thing that tells the screen
    // the settled rows are gone, and nothing else emits until the next transfer settles.
    let announced = manager.events.payloads("download:state");
    assert_eq!(
        announced.len(),
        4,
        "the cleared list never reached the screen"
    );
    let told: Vec<&str> = announced[3]
        .as_array()
        .expect("a state event carries the job list")
        .iter()
        .map(|job| job["id"].as_str().expect("id"))
        .collect();
    assert_eq!(told, [running.as_str()]);

    manager.downloads.cancel(&running).expect("cancelled");
    assert_eq!(
        settled(&manager.downloads, &running).state,
        DownloadState::Cancelled
    );
}
