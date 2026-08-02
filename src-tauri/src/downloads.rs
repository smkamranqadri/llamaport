use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::download::{
    self, Control, Phase, Progress, ProgressSink, Spec, CANCELLED, DEFAULT_FLUSH_EVERY,
    DEFAULT_PROGRESS_EVERY,
};
use crate::runner::Events;

/// A segment that has said nothing for this long has hung rather than slowed: its
/// siblings are still moving and the range is reissued.
const STALL_AFTER: Duration = Duration::from_secs(30);
const RETRY_BACKOFF: Duration = Duration::from_secs(1);

/// A limit below this is a mistake rather than a preference — a field read as KB/s, say.
/// The engine charges a 64 KB buffer at a time and sleeps out the shortfall, so a slower
/// limit parks a segment for longer than a second at a stretch with a cancel behind it.
const MIN_RATE_LIMIT: u64 = 64 * 1024;

const HOSTS: [&str; 2] = ["huggingface.co", "hf.co"];
const NOT_A_FILE_URL: &str = "this does not look like a Hugging Face file URL — expected \
     https://huggingface.co/{repo}/resolve/{ref}/{file}.gguf";

/// What the transfer is allowed to do, remembered between runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Options {
    pub segments: usize,
    /// Bytes per second across the whole transfer. `None` is unlimited.
    pub rate_limit: Option<u64>,
    pub verify: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            segments: 4,
            rate_limit: None,
            verify: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadState {
    Active,
    Complete,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJob {
    pub id: String,
    pub url: String,
    pub file_name: String,
    pub path: String,
    pub state: DownloadState,
    /// Absent until the engine has reported once.
    pub phase: Option<Phase>,
    pub completed: u64,
    pub total: Option<u64>,
    pub bytes_per_second: Option<f64>,
    pub error: Option<String>,
    pub started_secs: Option<u64>,
    pub finished_secs: Option<u64>,
}

impl DownloadJob {
    pub fn begin(id: &str, url: &str, dest: &Path) -> Self {
        Self {
            id: id.to_string(),
            url: url.to_string(),
            file_name: dest
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: dest.to_string_lossy().into_owned(),
            state: DownloadState::Active,
            phase: None,
            completed: 0,
            total: None,
            bytes_per_second: None,
            error: None,
            started_secs: now_secs(),
            finished_secs: None,
        }
    }
}

/// The name the file will be written under, and the reason it cannot be fetched at all.
///
/// Strict about the host on purpose: `https://huggingface.co@elsewhere.example/...` is a
/// URL whose authority is `elsewhere.example`, and matching on a prefix would accept it.
/// The engine follows the redirect to the CDN itself, so only the origin is named here.
pub fn file_name_for(url: &str) -> Result<String, String> {
    let rest = url
        .trim()
        .strip_prefix("https://")
        .ok_or_else(|| "a download URL must start with https://".to_string())?;

    let (host, path) = rest
        .split_once('/')
        .ok_or_else(|| NOT_A_FILE_URL.to_string())?;
    if !HOSTS.contains(&host.to_ascii_lowercase().as_str()) {
        return Err(NOT_A_FILE_URL.to_string());
    }

    let path = path.split(['?', '#']).next().unwrap_or_default();
    if !path.contains("/resolve/") {
        return Err(NOT_A_FILE_URL.to_string());
    }

    let name = path.rsplit('/').next().unwrap_or_default();
    if !name.to_ascii_lowercase().ends_with(".gguf") {
        return Err("only .gguf files can be downloaded into the models directory".to_string());
    }
    Ok(name.to_string())
}

/// Whether a request may start, and where it would land.
///
/// One transfer at a time: parallel large files compete for the same pipe and multiply
/// the failure surface, and refusing says so rather than quietly queueing behind a
/// transfer that has hours left.
pub fn admit(url: &str, models_dir: &Path, jobs: &[DownloadJob]) -> Result<PathBuf, String> {
    let file_name = file_name_for(url)?;

    if let Some(job) = jobs.iter().find(|j| j.state == DownloadState::Active) {
        if job.url == url {
            return Err(format!("{file_name} is already downloading"));
        }
        if job.file_name == file_name {
            return Err(format!(
                "{file_name} is already being downloaded from another URL"
            ));
        }
        return Err(format!(
            "{} is still downloading — this app downloads one file at a time",
            job.file_name
        ));
    }

    let dest = models_dir.join(&file_name);
    if dest.exists() {
        return Err(format!("{file_name} is already in the models directory"));
    }
    Ok(dest)
}

/// Whether the cancel has anything to signal. A download that has already settled is
/// nothing to cancel rather than an error — the screen and the transfer race, and the
/// screen loses often.
pub fn cancellable(jobs: &[DownloadJob], id: &str) -> Result<bool, String> {
    let job = jobs
        .iter()
        .find(|job| job.id == id)
        .ok_or_else(|| format!("no download {id}"))?;
    Ok(job.state == DownloadState::Active)
}

/// A cancellation reaches the caller as an error, and it is not one.
pub fn settle(result: Result<(), String>) -> (DownloadState, Option<String>) {
    match result {
        Ok(()) => (DownloadState::Complete, None),
        Err(cause) if cause == CANCELLED => (DownloadState::Cancelled, None),
        Err(cause) => (DownloadState::Failed, Some(cause)),
    }
}

pub fn spec_for(url: &str, dest: &Path, options: &Options) -> Spec {
    Spec {
        url: url.to_string(),
        dest: dest.to_path_buf(),
        segments: options.segments,
        stall_after: STALL_AFTER,
        retry_backoff: RETRY_BACKOFF,
        rate_limit: options
            .rate_limit
            .filter(|rate| *rate > 0)
            .map(|rate| rate.max(MIN_RATE_LIMIT)),
        verify: options.verify,
        progress_every: DEFAULT_PROGRESS_EVERY,
        flush_every: DEFAULT_FLUSH_EVERY,
    }
}

fn now_secs() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

struct Tracked {
    view: DownloadJob,
    control: Arc<Control>,
}

fn views(jobs: &[Tracked]) -> Vec<DownloadJob> {
    jobs.iter().map(|job| job.view.clone()).collect()
}

/// Run once a file has landed in the models directory, so whoever owns the catalog can
/// take the new model into account. The manager knows a rename succeeded and nothing else.
pub type Landed = Arc<dyn Fn() + Send + Sync>;

/// What a started job runs. The engine is the only implementation the app ships — a URL
/// this manager admits can only name Hugging Face, so nothing else reaches the
/// bookkeeping a transfer is wrapped in.
pub type Engine =
    Arc<dyn Fn(&Spec, &Control, &dyn ProgressSink) -> Result<(), String> + Send + Sync>;

pub struct Downloads {
    jobs: Arc<Mutex<Vec<Tracked>>>,
    events: Events,
    landed: Landed,
    engine: Engine,
    next: AtomicU64,
}

impl Downloads {
    pub fn new(events: Events, landed: Landed) -> Self {
        Self::with_engine(events, landed, Arc::new(download::download))
    }

    pub fn with_engine(events: Events, landed: Landed, engine: Engine) -> Self {
        Self {
            jobs: Arc::new(Mutex::new(Vec::new())),
            events,
            landed,
            engine,
            next: AtomicU64::new(1),
        }
    }

    pub fn snapshot(&self) -> Vec<DownloadJob> {
        views(&self.jobs.lock().expect("downloads lock"))
    }

    pub fn start(
        &self,
        url: &str,
        models_dir: &Path,
        options: &Options,
    ) -> Result<Vec<DownloadJob>, String> {
        let id = format!("dl-{}", self.next.fetch_add(1, Ordering::Relaxed));
        let control = Arc::new(Control::default());

        let (dest, snapshot) = {
            let mut jobs = self.jobs.lock().expect("downloads lock");
            let dest = admit(url, models_dir, &views(&jobs))?;
            std::fs::create_dir_all(models_dir).map_err(|e| e.to_string())?;
            jobs.push(Tracked {
                view: DownloadJob::begin(&id, url, &dest),
                control: control.clone(),
            });
            (dest, views(&jobs))
        };
        emit_state(&self.events, &self.jobs);

        let spec = spec_for(url, &dest, options);
        let jobs = self.jobs.clone();
        let events = self.events.clone();
        let landed = self.landed.clone();
        let engine = self.engine.clone();
        let sink = Sink {
            id: id.clone(),
            jobs: jobs.clone(),
            events: events.clone(),
        };

        // The transfer blocks for as long as it takes, which is hours.
        thread::spawn(move || {
            let outcome = engine(&spec, &control, &sink);
            let complete = outcome.is_ok();
            finish(&jobs, &id, outcome);
            // Ahead of the state event, so a screen that answers a finished transfer by
            // reading the catalog finds the file already in it.
            if complete {
                landed();
            }
            emit_state(&events, &jobs);
        });

        Ok(snapshot)
    }

    pub fn cancel(&self, id: &str) -> Result<Vec<DownloadJob>, String> {
        let jobs = self.jobs.lock().expect("downloads lock");
        let snapshot = views(&jobs);
        if cancellable(&snapshot, id)? {
            if let Some(job) = jobs.iter().find(|job| job.view.id == id) {
                job.control.cancel();
            }
        }
        Ok(snapshot)
    }

    /// Drops everything that has settled. The `.part` of a cancelled or failed transfer
    /// stays on disk, so starting the same URL again resumes rather than restarts.
    pub fn clear(&self) -> Vec<DownloadJob> {
        let snapshot = {
            let mut jobs = self.jobs.lock().expect("downloads lock");
            jobs.retain(|job| job.view.state == DownloadState::Active);
            views(&jobs)
        };
        emit_state(&self.events, &self.jobs);
        snapshot
    }
}

fn finish(jobs: &Mutex<Vec<Tracked>>, id: &str, outcome: Result<(), String>) {
    let (state, error) = settle(outcome);
    let mut jobs = jobs.lock().expect("downloads lock");
    let Some(job) = jobs.iter_mut().find(|job| job.view.id == id) else {
        return;
    };

    job.view.state = state;
    job.view.error = error;
    job.view.finished_secs = now_secs();
    job.view.bytes_per_second = None;
    if state == DownloadState::Complete {
        job.view.completed = job.view.total.unwrap_or(job.view.completed);
    }
}

fn emit_state(events: &Events, jobs: &Mutex<Vec<Tracked>>) {
    let snapshot = views(&jobs.lock().expect("downloads lock"));
    if let Ok(payload) = serde_json::to_value(snapshot) {
        events.emit("download:state", payload);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Reported<'a> {
    id: &'a str,
    #[serde(flatten)]
    progress: Progress,
}

/// Adapts the engine's reports onto the app's event bus, and keeps the job current so a
/// screen that opens mid-transfer starts from where it is rather than from nothing.
struct Sink {
    id: String,
    jobs: Arc<Mutex<Vec<Tracked>>>,
    events: Events,
}

impl ProgressSink for Sink {
    fn report(&self, progress: Progress) {
        {
            let mut jobs = self.jobs.lock().expect("downloads lock");
            if let Some(job) = jobs.iter_mut().find(|job| job.view.id == self.id) {
                job.view.phase = Some(progress.phase);
                job.view.completed = progress.completed;
                job.view.total = progress.total.or(job.view.total);
                job.view.bytes_per_second = progress.bytes_per_second;
            }
        }

        if let Ok(payload) = serde_json::to_value(Reported {
            id: &self.id,
            progress,
        }) {
            self.events.emit("download:progress", payload);
        }
    }
}
