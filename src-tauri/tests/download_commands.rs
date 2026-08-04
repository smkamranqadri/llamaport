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
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use llamaport_lib::download::{
    Control, Phase, Progress, ProgressSink, Spec, CANCELLED, DEFAULT_FLUSH_EVERY,
    DEFAULT_PROGRESS_EVERY,
};
use llamaport_lib::downloads::{
    admit, cancellable, file_name_for, normalized_rate, settle, spec_for, DownloadJob,
    DownloadState, Downloads, Engine, Options,
};
use llamaport_lib::runner::EventSink;

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

const OTHER: &str = "https://huggingface.co/a/b/resolve/main/Other-Q4_K_M.gguf";
const OTHER_FILE: &str = "Other-Q4_K_M.gguf";
const THIRD: &str = "https://huggingface.co/a/b/resolve/main/Third-Q4_K_M.gguf";

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
    /// The last thing written to `downloads.json`, which is what the next launch reads.
    history: Arc<Mutex<Vec<DownloadJob>>>,
}

impl Manager {
    fn history(&self) -> Vec<DownloadJob> {
        self.history.lock().expect("history lock").clone()
    }

    fn held(&self, id: &str) -> DownloadJob {
        self.downloads
            .snapshot()
            .into_iter()
            .find(|job| job.id == id)
            .expect("the job must stay tracked")
    }

    fn queue(&self) -> Vec<String> {
        self.downloads
            .snapshot()
            .into_iter()
            .filter(|job| job.state == DownloadState::Queued)
            .map(|job| job.id)
            .collect()
    }
}

fn manager(engine: Engine) -> Manager {
    let events = Arc::new(Recorder::default());
    let landings = Arc::new(AtomicU32::new(0));
    let counted = Arc::clone(&landings);
    let history = Arc::new(Mutex::new(Vec::new()));
    let written = Arc::clone(&history);
    let downloads = Downloads::with_engine(
        events.clone(),
        Arc::new(move || {
            counted.fetch_add(1, Ordering::Relaxed);
        }),
        engine,
    )
    .persisting_with(Arc::new(move |jobs| {
        *written.lock().expect("history lock") = jobs.to_vec();
    }));
    Manager {
        downloads,
        events,
        landings,
        history,
    }
}

/// Stands in for a transfer the test decides the length of, so a second request is made
/// while the first is provably still running rather than whenever the thread gets there.
fn wait_for(flag: &AtomicBool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !flag.load(Ordering::Relaxed) {
        assert!(
            Instant::now() < deadline,
            "the test never released the transfer"
        );
        thread::sleep(Duration::from_millis(5));
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

/// A transfer holds its own file against a second row over the same `.part`. What it no
/// longer holds is the app: another file is taken on and waits its turn.
#[test]
fn a_running_transfer_holds_its_own_file_and_not_the_whole_app() {
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
        &[job("dl-1", OTHER, OTHER_FILE, DownloadState::Active)],
    );
    assert_eq!(
        other,
        Ok(dir.join(FILE)),
        "a different file waits behind this one rather than being turned away"
    );
}

/// The same rule from the other side: a row already waiting owns its file too, and the
/// message has to say which line it is in or Resume is offered for something that is not
/// paused.
#[test]
fn a_file_already_in_the_queue_cannot_be_queued_again() {
    let dir = scratch("queue-dupe");
    let jobs = [
        job("dl-1", URL, FILE, DownloadState::Active),
        job("dl-2", OTHER, OTHER_FILE, DownloadState::Queued),
    ];

    let same = admit(OTHER, &dir, &jobs).expect_err("refused");
    assert!(same.contains("already in the queue"), "{same}");

    let renamed = admit(
        "https://huggingface.co/z/z/resolve/main/Other-Q4_K_M.gguf",
        &dir,
        &jobs,
    )
    .expect_err("refused");
    assert!(renamed.contains("under another URL"), "{renamed}");

    assert!(
        admit(THIRD, &dir, &jobs).is_ok(),
        "a third file is another place in the line, not a collision"
    );
}

#[test]
fn a_settled_transfer_does_not_hold_the_line() {
    let dir = scratch("settled");
    let history = [
        job("dl-1", URL, FILE, DownloadState::Failed),
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
fn pausing_a_download_that_is_no_longer_running_is_not_an_error() {
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
fn a_paused_transfer_is_not_reported_as_a_failure() {
    assert_eq!(settle(Ok(())), (DownloadState::Complete, None));
    assert_eq!(
        settle(Err(CANCELLED.to_string())),
        (DownloadState::Paused, None)
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
    assert!(!spec.verify);
}

/// The engine charges a 64 KB buffer at a time and sleeps out whatever the budget does not
/// cover, so a limit of a byte a second is an eighteen hour sleep between reads: the
/// transfer stops responding to a cancel and the app refuses every download behind it.
#[test]
fn a_rate_limit_too_low_to_transfer_at_is_raised_to_one_that_is() {
    for asked in [1, 100, 8_000] {
        let rate = normalized_rate(Some(asked)).expect("a limit was asked for");
        assert!(
            rate >= 64 * 1024,
            "{asked} bytes per second reached the engine as {rate}, which parks a buffer \
             for longer than a second"
        );
    }

    assert_eq!(
        normalized_rate(Some(0)),
        None,
        "a limit of zero is no limit, not a stalled transfer"
    );
    assert_eq!(normalized_rate(None), None);
    assert_eq!(
        normalized_rate(Some(10_000_000)),
        Some(10_000_000),
        "a limit above the floor is the user's to choose"
    );
}

/// A limit is set while watching the transfer it applies to, so reaching the store is not
/// enough: it has to reach the transfer that is already running.
#[test]
fn a_rate_limit_changed_mid_transfer_reaches_the_running_one() {
    let dir = scratch("live-rate");
    let observed = Arc::new(Mutex::new(None));
    let watched = Arc::clone(&observed);

    // Stands in for a transfer reading its budget as it goes, which is what the engine's
    // token bucket does on every charge.
    let engine: Engine = Arc::new(move |_: &Spec, control: &Control, _: &dyn ProgressSink| {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !control.cancelled() && Instant::now() < deadline {
            *watched.lock().expect("rate lock") = control.rate_limit();
            thread::sleep(Duration::from_millis(5));
        }
        Err(CANCELLED.to_string())
    });
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(
            URL,
            &dir,
            &Options {
                rate_limit: Some(10_000_000),
                ..Options::default()
            },
        )
        .expect("admitted");
    let running = started[0].id.clone();

    let seen = || *observed.lock().expect("rate lock");
    eventually("the stored limit never reached the transfer", || {
        seen() == Some(10_000_000)
    });

    manager.downloads.set_rate_limit(Some(2_000_000));
    eventually("the lowered limit never reached the transfer", || {
        seen() == Some(2_000_000)
    });

    manager.downloads.set_rate_limit(Some(1));
    eventually("the floor was not applied to a live change", || {
        seen() == Some(64 * 1024)
    });

    manager.downloads.set_rate_limit(None);
    eventually("lifting the limit never reached the transfer", || {
        seen().is_none()
    });

    manager.downloads.pause(&running).expect("paused");
    assert_eq!(
        settled(&manager.downloads, &running).state,
        DownloadState::Paused
    );
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
///
/// One at a time is unchanged. What changed is the answer to the second request: it is
/// taken on, it is kept away from the engine, and it goes the moment the pipe is free.
#[test]
fn a_second_url_waits_its_turn_and_starts_when_the_first_stops() {
    let dir = scratch("live-line");
    let reached = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&reached);

    let engine: Engine = Arc::new(
        move |spec: &Spec, control: &Control, _: &dyn ProgressSink| {
            seen.lock().expect("engine lock").push(spec.url.clone());
            wait_for_cancel(control)
        },
    );
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let running = started[0].id.clone();
    let urls = || reached.lock().expect("engine lock").clone();
    eventually("the transfer never reached the engine", || {
        urls().len() == 1
    });

    let queued = manager
        .downloads
        .start(OTHER, &dir, &Options::default())
        .expect("taken on rather than refused");
    assert_eq!(queued.len(), 2, "the waiting transfer has to be tracked");
    assert_eq!(queued[1].state, DownloadState::Queued);
    assert_eq!(
        queued[1].started_secs, None,
        "a job that has not started has no start time"
    );
    let waiting = queued[1].id.clone();

    assert_eq!(
        urls(),
        [URL],
        "a queued transfer must not reach the engine while another is running"
    );

    manager.downloads.pause(&running).expect("paused");
    assert_eq!(
        settled(&manager.downloads, &running).state,
        DownloadState::Paused
    );

    eventually("the queue never handed the pipe on", || urls().len() == 2);
    assert_eq!(urls()[1], OTHER);

    let promoted = manager.held(&waiting);
    assert_eq!(promoted.state, DownloadState::Active);
    assert!(
        promoted.started_secs.is_some(),
        "a transfer that is running started at some point"
    );

    manager.downloads.pause(&waiting).expect("paused");
    settled(&manager.downloads, &waiting);
}

/// Whatever ends the transfer at the head of the line hands the pipe on: it finished, it
/// failed, and a pause is covered where the queue is first proved. There is no outcome
/// that leaves the queue holding.
#[test]
fn a_transfer_that_ends_on_its_own_hands_the_pipe_to_the_queue() {
    for (label, outcome) in [
        ("complete", Ok(())),
        ("failed", Err("HTTP 404".to_string())),
    ] {
        let dir = scratch(&format!("queue-{label}"));
        let release = Arc::new(AtomicBool::new(false));
        let held = Arc::clone(&release);
        let reached = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&reached);

        let engine: Engine = Arc::new(
            move |spec: &Spec, control: &Control, _: &dyn ProgressSink| {
                seen.lock().expect("engine lock").push(spec.url.clone());
                if spec.url == URL {
                    wait_for(&held);
                    return outcome.clone();
                }
                wait_for_cancel(control)
            },
        );
        let manager = manager(engine);

        let started = manager
            .downloads
            .start(URL, &dir, &Options::default())
            .expect("admitted");
        let first = started[0].id.clone();
        let urls = || reached.lock().expect("engine lock").clone();
        eventually("the transfer never reached the engine", || {
            urls().len() == 1
        });

        let queued = manager
            .downloads
            .start(OTHER, &dir, &Options::default())
            .expect("taken on");
        let waiting = queued[1].id.clone();
        assert_eq!(queued[1].state, DownloadState::Queued, "{label}");

        release.store(true, Ordering::Relaxed);
        settled(&manager.downloads, &first);

        eventually(
            &format!("a {label} transfer never released the queue"),
            || urls().len() == 2,
        );
        assert_eq!(
            manager.held(&waiting).state,
            DownloadState::Active,
            "{label}"
        );

        manager.downloads.pause(&waiting).expect("paused");
        settled(&manager.downloads, &waiting);
    }
}

/// Discard is the other user-initiated stop, and it takes the same path as a pause: the
/// engine is signalled, the row goes on the settle path, and the queue moves up.
#[test]
fn discarding_the_running_transfer_hands_the_pipe_to_the_queue() {
    let dir = scratch("queue-discard");
    let reached = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&reached);

    let engine: Engine = Arc::new(
        move |spec: &Spec, control: &Control, _: &dyn ProgressSink| {
            seen.lock().expect("engine lock").push(spec.url.clone());
            wait_for_cancel(control)
        },
    );
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let running = started[0].id.clone();
    let urls = || reached.lock().expect("engine lock").clone();
    eventually("the transfer never reached the engine", || {
        urls().len() == 1
    });

    let queued = manager
        .downloads
        .start(OTHER, &dir, &Options::default())
        .expect("taken on");
    let waiting = queued[1].id.clone();

    manager.downloads.discard(&running).expect("discarded");
    eventually("the discarded row was never dropped", || {
        manager.downloads.snapshot().len() == 1
    });
    eventually("a discard never released the queue", || urls().len() == 2);
    assert_eq!(manager.held(&waiting).state, DownloadState::Active);

    manager.downloads.pause(&waiting).expect("paused");
    settled(&manager.downloads, &waiting);
}

/// A resume is a transfer starting, so it takes its turn like any other — at the back.
/// The list is the queue's order, and a row paused hours ago sits at an old index: served
/// from there it would jump ahead of everything added since.
#[test]
fn resuming_while_something_runs_joins_the_back_of_the_queue() {
    let dir = scratch("queue-resume");
    let engine: Engine =
        Arc::new(|_: &Spec, control: &Control, _: &dyn ProgressSink| wait_for_cancel(control));
    let manager = manager(engine);

    let first = manager
        .downloads
        .start(OTHER, &dir, &Options::default())
        .expect("admitted");
    let old = first[0].id.clone();
    manager.downloads.pause(&old).expect("paused");
    assert_eq!(
        settled(&manager.downloads, &old).state,
        DownloadState::Paused
    );

    let running = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let running = running
        .iter()
        .find(|job| job.url == URL)
        .expect("the job just started")
        .id
        .clone();

    let queued = manager
        .downloads
        .start(THIRD, &dir, &Options::default())
        .expect("taken on");
    let third = queued
        .iter()
        .find(|job| job.url == THIRD)
        .expect("just queued")
        .id
        .clone();

    let after = manager
        .downloads
        .resume(&old, &dir, &Options::default())
        .expect("a resume waits its turn rather than being refused");
    assert_eq!(
        after.iter().find(|job| job.id == old).map(|job| job.state),
        Some(DownloadState::Queued)
    );
    assert_eq!(
        manager.queue(),
        [third.as_str(), old.as_str()],
        "the resumed row was served ahead of one queued before it"
    );

    let again = manager
        .downloads
        .resume(&old, &dir, &Options::default())
        .expect_err("a row already in the line has nothing to resume");
    assert!(again.contains("already in the queue"), "{again}");

    manager.downloads.pause(&running).expect("paused");
    eventually("the queue never started the row behind it", || {
        manager.held(&third).state == DownloadState::Active
    });
    assert_eq!(manager.held(&old).state, DownloadState::Queued);

    manager.downloads.pause(&third).expect("paused");
    eventually("the last of the queue never started", || {
        manager.held(&old).state == DownloadState::Active
    });
    manager.downloads.pause(&old).expect("paused");
    settled(&manager.downloads, &old);
}

/// A queued job carries the terms it was admitted on, because it starts on a thread with
/// no caller to ask. The rate limit is the one term that is live, and a limit set while
/// three files wait has to be the one they start under — otherwise it is honoured by
/// nothing the user can see.
#[test]
fn a_limit_set_while_a_job_waits_is_the_one_it_starts_under() {
    let dir = scratch("queue-rate");
    let observed = Arc::new(Mutex::new(None));
    let watched = Arc::clone(&observed);

    let engine: Engine = Arc::new(
        move |spec: &Spec, control: &Control, _: &dyn ProgressSink| {
            if spec.url == OTHER {
                *watched.lock().expect("rate lock") = Some(control.rate_limit());
            }
            wait_for_cancel(control)
        },
    );
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let running = started[0].id.clone();
    manager
        .downloads
        .start(OTHER, &dir, &Options::default())
        .expect("taken on");

    manager.downloads.set_rate_limit(Some(2_000_000));
    manager.downloads.pause(&running).expect("paused");

    eventually("the queued transfer never started", || {
        observed.lock().expect("rate lock").is_some()
    });
    assert_eq!(
        *observed.lock().expect("rate lock"),
        Some(Some(2_000_000)),
        "the waiting transfer started under the limit it was admitted with"
    );
}

/// The check that admitted a job ran against a models directory that has had hours to
/// change. A file that landed in the meantime is not something to write over.
#[test]
fn a_file_that_landed_while_a_job_waited_is_not_overwritten() {
    let dir = scratch("queue-landed");
    let release = Arc::new(AtomicBool::new(false));
    let held = Arc::clone(&release);

    let engine: Engine = Arc::new(move |spec: &Spec, _: &Control, _: &dyn ProgressSink| {
        assert_eq!(
            spec.url, URL,
            "a job whose file is already there must never reach the engine"
        );
        wait_for(&held);
        Ok(())
    });
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let running = started[0].id.clone();

    let queued = manager
        .downloads
        .start(OTHER, &dir, &Options::default())
        .expect("taken on");
    let waiting = queued[1].id.clone();

    fs::write(dir.join(OTHER_FILE), b"landed some other way").expect("seed");
    release.store(true, Ordering::Relaxed);
    settled(&manager.downloads, &running);

    eventually("the waiting job was never resolved either way", || {
        manager.held(&waiting).state.finished()
    });
    let blocked = manager.held(&waiting);
    assert_eq!(blocked.state, DownloadState::Failed);
    assert!(
        blocked
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("already in the models directory"),
        "{:?}",
        blocked.error
    );
    assert_eq!(
        fs::read(dir.join(OTHER_FILE)).expect("read"),
        b"landed some other way",
        "the queued transfer wrote over a file that was already there"
    );
}

/// Clearing drops the history. A queued row is not history — it is a place in a line, and
/// it has to survive both the list and the file the list is written to.
#[test]
fn clearing_leaves_the_queue_alone() {
    let dir = scratch("queue-clear");
    let engine: Engine = Arc::new(|spec: &Spec, control: &Control, _: &dyn ProgressSink| {
        if spec.url == THIRD {
            return Err("HTTP 500".to_string());
        }
        wait_for_cancel(control)
    });
    let manager = manager(engine);

    let done = manager
        .downloads
        .start(THIRD, &dir, &Options::default())
        .expect("admitted");
    let done = done[0].id.clone();
    assert_eq!(
        settled(&manager.downloads, &done).state,
        DownloadState::Failed
    );

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let running = started
        .iter()
        .find(|job| job.url == URL)
        .expect("just started")
        .id
        .clone();
    let queued = manager
        .downloads
        .start(OTHER, &dir, &Options::default())
        .expect("taken on");
    let waiting = queued
        .iter()
        .find(|job| job.url == OTHER)
        .expect("just queued")
        .id
        .clone();

    let left = manager.downloads.clear();
    assert_eq!(
        left.iter().map(|job| job.id.as_str()).collect::<Vec<_>>(),
        [running.as_str(), waiting.as_str()],
        "clearing the history took the queue with it"
    );
    let kept = manager.history();
    assert_eq!(
        kept.iter().map(|job| job.id.as_str()).collect::<Vec<_>>(),
        [waiting.as_str()],
        "the queue was cleared out of the file the next launch reads"
    );

    manager.downloads.pause(&running).expect("paused");
    settled(&manager.downloads, &running);
    eventually("the queue never started", || {
        manager.held(&waiting).state == DownloadState::Active
    });
    manager.downloads.pause(&waiting).expect("paused");
    settled(&manager.downloads, &waiting);
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
fn pausing_reaches_the_transfer_and_settles_the_job() {
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
        .pause("dl-9")
        .expect_err("an id that was never started is a mistake, not a no-op");
    assert!(unknown.contains("no download dl-9"), "{unknown}");

    manager.downloads.pause(&id).expect("paused");

    let job = settled(&manager.downloads, &id);
    assert_eq!(job.state, DownloadState::Paused);
    assert_eq!(job.error, None, "a pause is not a failure");

    eventually("the settled job was never announced", || {
        manager.events.payloads("download:state").len() == 2
    });
    assert_eq!(manager.landings.load(Ordering::Relaxed), 0);

    let again = manager
        .downloads
        .pause(&id)
        .expect("pausing a transfer that has already stopped is nothing to do");
    assert_eq!(again.len(), 1);
    assert_eq!(manager.downloads.snapshot()[0].state, DownloadState::Paused);
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

    manager.downloads.pause(&running).expect("paused");
    assert_eq!(
        settled(&manager.downloads, &running).state,
        DownloadState::Paused
    );
}

/// The bytes were never the thing that was lost. A stopped transfer keeps its `.part` and
/// its sidecar, so what a pause has to preserve is the row that points at them — and a
/// resume has to continue that row rather than open a second one for the same file.
#[test]
fn pausing_settles_the_job_and_resuming_continues_the_same_one() {
    let dir = scratch("pause");
    let runs = Arc::new(AtomicU32::new(0));
    let counted = Arc::clone(&runs);

    let engine: Engine = Arc::new(move |_: &Spec, control: &Control, _: &dyn ProgressSink| {
        counted.fetch_add(1, Ordering::Relaxed);
        wait_for_cancel(control)
    });
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let id = started[0].id.clone();
    eventually("the transfer never reached the engine", || {
        runs.load(Ordering::Relaxed) == 1
    });

    manager.downloads.pause(&id).expect("paused");
    let paused = settled(&manager.downloads, &id);
    assert_eq!(paused.state, DownloadState::Paused);
    assert_eq!(paused.error, None, "a pause is not a failure");

    let after = manager
        .downloads
        .resume(&id, &dir, &Options::default())
        .expect("resumed");
    assert_eq!(
        after.len(),
        1,
        "resuming opened a second row for a file that already had one"
    );
    assert_eq!(after[0].id, id, "the resumed transfer is the same job");
    assert_eq!(after[0].state, DownloadState::Active);
    eventually("the resume never reached the engine", || {
        runs.load(Ordering::Relaxed) == 2
    });

    let second = manager
        .downloads
        .resume(&id, &dir, &Options::default())
        .expect_err("a transfer that is already running has nothing to resume");
    assert!(second.contains("already downloading"), "{second}");

    manager.downloads.pause(&id).expect("paused");
    settled(&manager.downloads, &id);
}

/// A paused transfer is unfinished business, not history. Starting its URL again would
/// open a second row over the same `.part`, and clearing the finished rows would throw
/// away bytes the user is waiting to continue.
#[test]
fn a_paused_transfer_holds_its_place_against_both_a_restart_and_a_clear() {
    let dir = scratch("paused-line");

    let blocked = admit(URL, &dir, &[job("dl-1", URL, FILE, DownloadState::Paused)])
        .expect_err("the file already has a row");
    assert!(blocked.contains("paused"), "{blocked}");

    let elsewhere = admit(
        "https://huggingface.co/a/b/resolve/main/Other-Q4_K_M.gguf",
        &dir,
        &[job("dl-1", URL, FILE, DownloadState::Paused)],
    );
    assert!(
        elsewhere.is_ok(),
        "a paused transfer occupies its own file, not the whole app: {elsewhere:?}"
    );

    let engine: Engine =
        Arc::new(|_: &Spec, control: &Control, _: &dyn ProgressSink| wait_for_cancel(control));
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let id = started[0].id.clone();
    manager.downloads.pause(&id).expect("paused");
    settled(&manager.downloads, &id);

    let left = manager.downloads.clear();
    assert_eq!(
        left.iter().map(|job| job.id.as_str()).collect::<Vec<_>>(),
        [id.as_str()],
        "clearing the history took a paused transfer with it"
    );
}

/// Discard has to outlive the engine's parting write.
///
/// `Control::cancel` returns while the transfer is still writing, and a transfer asked to
/// stop still flushes its sidecar before it returns. A discard that only deletes where it
/// was asked leaves both files back on disk moments later.
///
/// What this pins is the delete on the settle path, not the absence of an earlier one:
/// deleting eagerly as well is wasteful rather than wrong, and this passes either way.
#[test]
fn discarding_takes_the_bytes_only_once_the_engine_has_let_go() {
    let dir = scratch("discard-running");
    let part = dir.join(format!("{FILE}.part"));
    let sidecar = dir.join(format!("{FILE}.part.json"));

    let engine: Engine = Arc::new(|spec: &Spec, control: &Control, _: &dyn ProgressSink| {
        let part = PathBuf::from(format!("{}.part", spec.dest.display()));
        let sidecar = PathBuf::from(format!("{}.part.json", spec.dest.display()));
        fs::write(&part, b"bytes so far").expect("seed part");
        fs::write(&sidecar, b"{}").expect("seed sidecar");

        let outcome = wait_for_cancel(control);

        // The flush on the way out, which is what the engine really does: a transfer that
        // is asked to stop still writes its sidecar before it returns.
        thread::sleep(Duration::from_millis(50));
        fs::write(&part, b"one last flush").expect("flush part");
        fs::write(&sidecar, b"{}").expect("flush sidecar");
        outcome
    });
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let id = started[0].id.clone();
    eventually("the transfer never wrote its part file", || part.exists());

    manager.downloads.discard(&id).expect("discarded");

    eventually("the discarded job was never dropped", || {
        manager.downloads.snapshot().is_empty()
    });
    assert!(
        !part.exists(),
        "the engine's parting write put the part file back, so the delete ran too early"
    );
    assert!(!sidecar.exists(), "the sidecar outlived the transfer");
}

/// Nothing is running, so there is no writer to wait for — but the files are still there,
/// and they are the whole reason the row exists.
#[test]
fn discarding_a_paused_transfer_takes_its_bytes_too() {
    let dir = scratch("discard-paused");
    let part = dir.join(format!("{FILE}.part"));
    let sidecar = dir.join(format!("{FILE}.part.json"));

    let engine: Engine = Arc::new(|spec: &Spec, control: &Control, _: &dyn ProgressSink| {
        fs::write(format!("{}.part", spec.dest.display()), b"bytes").expect("seed part");
        fs::write(format!("{}.part.json", spec.dest.display()), b"{}").expect("seed sidecar");
        wait_for_cancel(control)
    });
    let manager = manager(engine);

    let started = manager
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let id = started[0].id.clone();
    eventually("the transfer never wrote its part file", || part.exists());
    manager.downloads.pause(&id).expect("paused");
    assert_eq!(
        settled(&manager.downloads, &id).state,
        DownloadState::Paused
    );

    let left = manager.downloads.discard(&id).expect("discarded");
    assert!(left.is_empty(), "the discarded row is still listed");
    assert!(!part.exists() && !sidecar.exists());

    // The file is gone from the app's account of it as well as from the disk, so the URL
    // is startable again rather than being held by a row that no longer means anything.
    assert!(admit(URL, &dir, &manager.downloads.snapshot()).is_ok());
}

fn seed_partial(dir: &std::path::Path, file: &str, url: &str, completed: u64, part_len: u64) {
    fs::write(
        dir.join(format!("{file}.part")),
        vec![0u8; part_len as usize],
    )
    .expect("seed part");
    fs::write(
        dir.join(format!("{file}.part.json")),
        format!(
            r#"{{"sourceUrl":"{url}","total":64000,"etag":null,"segments":[
                {{"start":0,"end":31999,"completed":{completed}}},
                {{"start":32000,"end":63999,"completed":0}}]}}"#
        ),
    )
    .expect("seed sidecar");
}

/// The app restarts knowing nothing, and the bytes are still on the disk where the last
/// run left them. Nothing else recovers a partial fetched by a build that had no history
/// file at all — which on the first run of this feature is every partial there is.
#[test]
fn partials_left_on_disk_are_adopted_as_paused_transfers() {
    let dir = scratch("adopt");
    seed_partial(&dir, FILE, URL, 12_000, 64_000);

    let engine: Engine = Arc::new(|_: &Spec, _: &Control, _: &dyn ProgressSink| Ok(()));
    let manager = manager(engine);

    let adopted = manager.downloads.adopt(&dir);
    assert_eq!(adopted.len(), 1, "the partial on disk was not found");

    let job = &adopted[0];
    assert_eq!(job.state, DownloadState::Paused);
    assert_eq!(
        job.url, URL,
        "the sidecar is the only record of where it came from"
    );
    assert_eq!(job.file_name, FILE);
    assert_eq!(job.path, dir.join(FILE).to_string_lossy());
    assert_eq!(job.completed, 12_000);
    assert_eq!(job.total, Some(64_000));
    assert!(job.resumable);

    assert_eq!(
        manager.downloads.adopt(&dir).len(),
        1,
        "adopting twice listed the same file twice"
    );

    // The row is a real one, not a placeholder: it holds the line and it can be discarded.
    let blocked = admit(URL, &dir, &manager.downloads.snapshot()).expect_err("held");
    assert!(blocked.contains("paused"), "{blocked}");

    manager
        .downloads
        .discard(&job.id.clone())
        .expect("discarded");
    assert!(!dir.join(format!("{FILE}.part")).exists());
    assert!(!dir.join(format!("{FILE}.part.json")).exists());
}

/// A sidecar whose `.part` is gone describes bytes that are not there. It is listed
/// anyway, because it is junk occupying the models directory and hiding it is how it got
/// to be a problem — but it must not offer a resume it cannot honour.
#[test]
fn an_unresumable_partial_is_listed_without_pretending_it_can_continue() {
    let dir = scratch("adopt-orphan");
    seed_partial(&dir, FILE, URL, 12_000, 64_000);
    fs::remove_file(dir.join(format!("{FILE}.part"))).expect("remove part");

    let engine: Engine = Arc::new(|_: &Spec, _: &Control, _: &dyn ProgressSink| Ok(()));
    let manager = manager(engine);

    let adopted = manager.downloads.adopt(&dir);
    assert_eq!(adopted.len(), 1);
    assert_eq!(adopted[0].state, DownloadState::Paused);
    assert!(
        !adopted[0].resumable,
        "there are no bytes beside this sidecar to continue from"
    );
}

/// The history file is the only account of what finished, and a restored row has to be
/// indistinguishable from one this process settled itself.
#[test]
fn restoring_seeds_the_history_without_colliding_with_new_ids() {
    let dir = scratch("restore");
    let engine: Engine = Arc::new(|_: &Spec, _: &Control, _: &dyn ProgressSink| Ok(()));
    let manager = manager(engine);

    manager.downloads.restore(
        vec![
            job("dl-1", URL, FILE, DownloadState::Complete),
            job(
                "dl-7",
                "https://huggingface.co/a/b/resolve/main/x.gguf",
                "x.gguf",
                DownloadState::Failed,
            ),
            job(
                "dl-9",
                "https://huggingface.co/a/b/resolve/main/y.gguf",
                "y.gguf",
                DownloadState::Active,
            ),
        ],
        &dir,
    );

    let restored = manager.downloads.snapshot();
    assert_eq!(
        restored.iter().map(|j| j.id.as_str()).collect::<Vec<_>>(),
        ["dl-1", "dl-7"],
        "only what finished belongs in the history; a transfer cannot survive the process \
         that was running it"
    );

    let started = manager
        .downloads
        .start(
            "https://huggingface.co/a/b/resolve/main/z.gguf",
            &dir,
            &Options::default(),
        )
        .expect("admitted");
    let fresh = started.last().expect("the job just started");
    assert!(
        !["dl-1", "dl-7"].contains(&fresh.id.as_str()),
        "a new transfer took an id the restored history already uses: {}",
        fresh.id
    );
}

/// A sidecar is a file in a folder, not a promise. Anything that can write to the models
/// directory can put one there, and an adopted row wears a model's filename and a
/// half-finished progress bar — which is the opposite of the suspicion a pasted URL earns.
///
/// `admit` is the only place a URL is checked, and `resume` does not go through it. So the
/// check has to happen on the way back in as well, or Resume is an unvalidated fetch.
#[test]
fn a_partial_naming_somewhere_other_than_hugging_face_cannot_be_resumed() {
    let dir = scratch("hostile-sidecar");
    seed_partial(
        &dir,
        FILE,
        "https://attacker.example/collect?u=victim",
        12_000,
        64_000,
    );

    let engine: Engine = Arc::new(|_: &Spec, _: &Control, _: &dyn ProgressSink| {
        panic!("the engine must never be reached for an unvalidated URL")
    });
    let manager = manager(engine);

    let adopted = manager.downloads.adopt(&dir);
    assert_eq!(
        adopted.len(),
        1,
        "it is still listed, so it can be discarded"
    );
    assert!(
        !adopted[0].resumable,
        "the screen offers Resume on `resumable`, so this is what stops the fetch"
    );

    let refused = manager
        .downloads
        .resume(&adopted[0].id, &dir, &Options::default())
        .expect_err("a command is reachable without the button");
    assert!(
        refused.contains("Hugging Face") || refused.contains("huggingface"),
        "{refused}"
    );
}

/// A queued row is the only unfinished work with nothing on disk describing it — no
/// `.part`, no sidecar — so the history file is the only thing that can carry it across a
/// restart. It comes back Paused: it has no bytes to prove it was ever this app's doing,
/// and launching the app is not consent to fetch a URL that was read off disk.
#[test]
fn a_queued_row_comes_back_as_a_paused_row_after_a_restart() {
    let dir = scratch("queue-restart");
    let engine: Engine =
        Arc::new(|_: &Spec, control: &Control, _: &dyn ProgressSink| wait_for_cancel(control));
    let first = manager(engine);

    let started = first
        .downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let running = started[0].id.clone();
    let queued = first
        .downloads
        .start(OTHER, &dir, &Options::default())
        .expect("taken on");
    let waiting = queued[1].id.clone();

    let written = first.history();
    assert_eq!(
        written
            .iter()
            .map(|job| job.id.as_str())
            .collect::<Vec<_>>(),
        [waiting.as_str()],
        "the running transfer belongs to its `.part`, not to the history file"
    );

    first.downloads.pause(&running).expect("paused");
    settled(&first.downloads, &running);

    // The next launch, which knows only what is on disk.
    let runs = Arc::new(AtomicU32::new(0));
    let counted = Arc::clone(&runs);
    let engine: Engine = Arc::new(move |_: &Spec, control: &Control, _: &dyn ProgressSink| {
        counted.fetch_add(1, Ordering::Relaxed);
        wait_for_cancel(control)
    });
    let next = manager(engine);
    next.downloads.restore(written, &dir);

    let restored = next.downloads.snapshot();
    assert_eq!(restored.len(), 1, "the queue did not survive the restart");
    assert_eq!(restored[0].id, waiting);
    assert_eq!(restored[0].state, DownloadState::Paused);
    assert_eq!(restored[0].url, OTHER);
    assert_eq!(restored[0].file_name, OTHER_FILE);
    assert_eq!(restored[0].path, dir.join(OTHER_FILE).to_string_lossy());
    assert_eq!(restored[0].completed, 0);
    assert_eq!(restored[0].total, None);
    assert!(
        restored[0].resumable,
        "the screen draws Resume from this, and there is nothing wrong with the row"
    );
    assert_eq!(
        runs.load(Ordering::Relaxed),
        0,
        "restoring must not fetch anything on its own"
    );

    next.downloads
        .resume(&waiting, &dir, &Options::default())
        .expect("resumed");
    eventually("the restored row could not be continued", || {
        runs.load(Ordering::Relaxed) == 1
    });
    next.downloads.pause(&waiting).expect("paused");
    settled(&next.downloads, &waiting);
}

/// A queued row is not always a row that never started. Resuming into the queue carries a
/// `.part` and its bytes along, and the restored row holds that file's path — which is
/// exactly what `adopt` treats as already tracked, so it leaves the partial alone. Nothing
/// else will correct the figure, which makes zeroing it the only account of those bytes
/// saying they are not there.
#[test]
fn a_queued_row_waiting_over_a_partial_comes_back_reporting_it() {
    let dir = scratch("queue-restart-bytes");
    seed_partial(&dir, FILE, URL, 12_000, 64_000);

    let engine: Engine = Arc::new(|_: &Spec, _: &Control, _: &dyn ProgressSink| Ok(()));
    let manager = manager(engine);

    let mut waiting = job("dl-1", URL, FILE, DownloadState::Queued);
    waiting.completed = 12_000;
    waiting.total = Some(64_000);
    manager.downloads.restore(vec![waiting], &dir);
    manager.downloads.adopt(&dir);

    let restored = manager.downloads.snapshot();
    assert_eq!(
        restored.len(),
        1,
        "the restored row and the partial are the same download, not two"
    );
    assert_eq!(restored[0].state, DownloadState::Paused);
    assert_eq!(
        restored[0].completed, 12_000,
        "the screen was told the bytes on disk are not there"
    );
    assert_eq!(restored[0].total, Some(64_000));
}

/// A queued row comes back Paused, and Paused was not something the history file kept — so
/// the queue survived one restart and the next write to that file dropped it. There is no
/// `.part` and no sidecar behind a row that never started, so nothing else remembers it.
#[test]
fn a_queue_that_came_back_once_is_still_there_the_next_time() {
    let dir = scratch("queue-restart-twice");
    let engine: Engine = Arc::new(|_: &Spec, _: &Control, _: &dyn ProgressSink| Ok(()));

    let first = manager(engine.clone());
    first
        .downloads
        .restore(vec![job("dl-1", URL, FILE, DownloadState::Queued)], &dir);
    assert_eq!(first.downloads.snapshot()[0].state, DownloadState::Paused);

    // Anything at all that rewrites the history file.
    first.downloads.clear();
    let written = first.history();
    assert_eq!(
        written.len(),
        1,
        "the restored queue was dropped by the only file that remembers it"
    );

    let next = manager(engine);
    next.downloads.restore(written, &dir);
    assert_eq!(
        next.downloads.snapshot().len(),
        1,
        "the queue survived one restart and not two"
    );
}

/// The other half of that rule, which is what keeps it from swallowing everything. A
/// transfer with bytes on disk is described by the sidecar beside them, and a sidecar whose
/// `.part` is gone still owns its row — taking either into the history file would shadow
/// `adopt`, which is the only thing that reads what is actually there.
#[test]
fn a_transfer_the_disk_describes_is_left_to_the_disk() {
    let dir = scratch("paused-not-persisted");
    let engine: Engine = Arc::new(|_: &Spec, _: &Control, _: &dyn ProgressSink| Ok(()));
    let manager = manager(engine);

    seed_partial(&dir, FILE, URL, 12_000, 64_000);
    seed_partial(&dir, OTHER_FILE, OTHER, 5_000, 64_000);
    fs::remove_file(dir.join(format!("{OTHER_FILE}.part"))).expect("remove part");

    let adopted = manager.downloads.adopt(&dir);
    assert_eq!(adopted.len(), 2);
    assert!(adopted.iter().all(|job| job.state == DownloadState::Paused));

    manager.downloads.clear();
    assert!(
        manager.history().is_empty(),
        "a transfer the disk already accounts for was copied into the history file too"
    );
}

/// The history file sits in a directory anything with write access can plant a row in, and
/// a queued row is a URL and nothing else — there are no bytes on disk to contradict it.
/// It is dropped rather than kept for discarding: an adopted partial is listed because its
/// bytes are occupying the models directory, and this one is occupying nothing.
#[test]
fn a_queued_row_naming_somewhere_other_than_hugging_face_is_not_restored() {
    let dir = scratch("queue-hostile");
    let engine: Engine = Arc::new(|_: &Spec, _: &Control, _: &dyn ProgressSink| {
        panic!("the engine must never be reached for an unvalidated URL")
    });
    let manager = manager(engine);

    manager.downloads.restore(
        vec![job(
            "dl-1",
            "https://attacker.example/collect?u=victim",
            FILE,
            DownloadState::Queued,
        )],
        &dir,
    );

    assert!(
        manager.downloads.snapshot().is_empty(),
        "a planted queue entry became a row with a Resume button"
    );
}

/// `path` is rebuilt from the models directory so a stored one cannot aim a write — but it
/// is rebuilt by joining `file_name`, which came off the same untrusted file. A name that
/// is not a name reaches back out of the directory the join was meant to confine it to.
#[test]
fn a_restored_row_cannot_name_a_file_outside_the_models_directory() {
    let dir = scratch("hostile-name");
    let engine: Engine = Arc::new(|_: &Spec, _: &Control, _: &dyn ProgressSink| Ok(()));
    let manager = manager(engine);

    for name in [
        "../../../../Users/victim/Library/LaunchAgents/com.evil.plist",
        "nested/Model-Q4_K_M.gguf",
        "..",
        "",
    ] {
        let mut hostile = job("dl-1", URL, FILE, DownloadState::Failed);
        hostile.file_name = name.to_string();
        manager.downloads.restore(vec![hostile], &dir);
        assert!(
            manager.downloads.snapshot().is_empty(),
            "{name} was taken back in and joined onto the models directory"
        );
    }
}

/// The history file is on disk beside the config, and `path` in it decides what a resume
/// writes and what a discard deletes. Trusting it makes those arbitrary; the destination
/// is derived from the models directory instead, so the stored value cannot aim them.
#[test]
fn a_restored_row_cannot_point_the_app_outside_the_models_directory() {
    let dir = scratch("hostile-history");
    let engine: Engine = Arc::new(|_: &Spec, _: &Control, _: &dyn ProgressSink| Ok(()));
    let manager = manager(engine);

    let mut hostile = job("dl-1", URL, FILE, DownloadState::Failed);
    hostile.path = "/Users/victim/Library/LaunchAgents/com.evil.plist".into();
    manager.downloads.restore(vec![hostile], &dir);

    let restored = manager.downloads.snapshot();
    assert_eq!(
        restored[0].path,
        dir.join(FILE).to_string_lossy(),
        "a path read off disk must not survive into anything that touches the filesystem"
    );
}
