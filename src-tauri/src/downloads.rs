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

/// The only hosts anything in this app will fetch from. One list, because a second
/// would drift from it: `hub::owner_of` reads the same URLs this validates.
pub(crate) const HOSTS: [&str; 2] = ["huggingface.co", "hf.co"];
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

/// `Paused` is what a stopped transfer has always been: cancelling leaves the `.part` and
/// the sidecar on disk and starting the same URL again resumes from them. Naming it that
/// is what lets the row survive a restart and offer a button rather than an apology.
///
/// There is no `Discarded`: discarding removes the row along with the bytes.
///
/// `Queued` is waiting for the pipe. One invariant governs it: nothing `Active` and
/// something `Queued` means the head of the queue starts, and every path a transfer can
/// settle on goes through it. So pausing the running file starts the next one — there is
/// no way to stop the app short of emptying the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadState {
    Active,
    Queued,
    Paused,
    Complete,
    Failed,
}

impl DownloadState {
    /// Whether this job is over. History is exactly what is finished — a paused transfer
    /// is unfinished business that lives on disk, not a record of something that happened.
    pub fn finished(self) -> bool {
        matches!(self, DownloadState::Complete | DownloadState::Failed)
    }
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
    /// Whether the bytes this row points at can be continued. Only ever false on a paused
    /// transfer adopted from a `.part` that no longer holds what its sidecar claims.
    pub resumable: bool,
    /// Who published it, read off the URL. Derived here rather than in the window so one
    /// place knows the shape of a download URL — the window would have to parse it a
    /// second way, and the two would drift.
    pub owner: Option<String>,
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
            resumable: true,
            owner: crate::hub::owner_of(url),
        }
    }

    /// A transfer this app never started, rebuilt from the `.part` a previous run left in
    /// the models directory. The sidecar is the only account of it there is.
    pub fn adopted(id: &str, dest: &Path, partial: &download::Partial) -> Self {
        let mut job = Self::begin(id, &partial.source_url, dest);
        job.state = DownloadState::Paused;
        job.completed = partial.completed;
        job.total = Some(partial.total);
        // A sidecar is a file anything can write. One naming somewhere this app would
        // never have downloaded from is listed so it can be discarded, but never offered
        // a Resume — the screen draws that button from this flag.
        job.resumable = partial.resumable && file_name_for(&partial.source_url).is_ok();
        job.started_secs = std::fs::metadata(download::sidecar_path(dest))
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
            .map(|since| since.as_secs());
        job
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

/// Whether a name that came off disk is a file name and nothing else.
///
/// Every destination the app writes, renames or deletes is a file name joined onto the
/// models directory, and `join("../evil")` lands outside it. The history file is where such
/// a name can be planted, so it is checked there rather than trusted for having been ours.
fn plain_file_name(name: &str) -> bool {
    Path::new(name).file_name().and_then(|only| only.to_str()) == Some(name)
}

/// Why an unfinished row already holding this file turns a request away. Only ever called
/// with a row that is Active, Queued or Paused, which is what makes the last arm honest.
fn held_by(job: &DownloadJob, url: &str) -> String {
    let same_url = job.url == url;
    match job.state {
        DownloadState::Paused => format!(
            "{} is paused — resume it from the list below rather than starting it again",
            job.file_name
        ),
        DownloadState::Queued if same_url => format!("{} is already in the queue", job.file_name),
        DownloadState::Queued => format!(
            "{} is already in the queue under another URL",
            job.file_name
        ),
        _ if same_url => format!("{} is already downloading", job.file_name),
        _ => format!(
            "{} is already being downloaded from another URL",
            job.file_name
        ),
    }
}

/// Whether a request may be taken on at all, and where it would land.
///
/// It no longer decides whether the transfer starts now: a second request queues behind the
/// first rather than being refused. What is still refused is a second row over one file —
/// whether the row holding it is running, waiting or paused, the two would fight over the
/// same `.part`.
pub fn admit(url: &str, models_dir: &Path, jobs: &[DownloadJob]) -> Result<PathBuf, String> {
    let file_name = file_name_for(url)?;

    if let Some(job) = jobs
        .iter()
        .filter(|j| !j.state.finished())
        .find(|j| j.url == url || j.file_name == file_name)
    {
        return Err(held_by(job, url));
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

/// Whether a job may be put back on the engine. A resume is a transfer starting, so it
/// takes its turn the same way a pasted URL does — it runs now if the pipe is free and
/// queues if it is not. What is refused is only a row with nothing to resume.
pub fn resumable(jobs: &[DownloadJob], id: &str) -> Result<(), String> {
    let job = jobs
        .iter()
        .find(|job| job.id == id)
        .ok_or_else(|| format!("no download {id}"))?;

    if job.state == DownloadState::Active {
        return Err(format!("{} is already downloading", job.file_name));
    }
    if job.state == DownloadState::Queued {
        return Err(format!("{} is already in the queue", job.file_name));
    }
    if job.state == DownloadState::Complete {
        return Err(format!("{} has already been downloaded", job.file_name));
    }
    Ok(())
}

/// A cancellation reaches the caller as an error, and it is not one — it is a pause, and
/// the bytes it stopped on are still on disk.
pub fn settle(result: Result<(), String>) -> (DownloadState, Option<String>) {
    match result {
        Ok(()) => (DownloadState::Complete, None),
        Err(cause) if cause == CANCELLED => (DownloadState::Paused, None),
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
        verify: options.verify,
        progress_every: DEFAULT_PROGRESS_EVERY,
        flush_every: DEFAULT_FLUSH_EVERY,
    }
}

/// What a stored limit means to the engine.
///
/// The floor is policy rather than mechanism, which is why it lives here: the engine
/// honours whatever it is told, and the tests rely on being able to tell it a byte a
/// second. What a user can ask for through the app is bounded here instead.
pub fn normalized_rate(rate: Option<u64>) -> Option<u64> {
    rate.filter(|rate| *rate > 0)
        .map(|rate| rate.max(MIN_RATE_LIMIT))
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
    /// Set before the cancel that stops a discarded transfer, and read on the settle path
    /// where the engine has already let the `.part` go.
    discarding: bool,
    /// The terms this job was taken on. A queued transfer starts on a thread that has no
    /// caller to ask, so what it runs under has to have been kept when it was admitted.
    options: Options,
}

fn views(jobs: &[Tracked]) -> Vec<DownloadJob> {
    jobs.iter().map(|job| job.view.clone()).collect()
}

/// Whether `downloads.json` is the only thing that remembers this row.
///
/// A transfer that moved bytes is described by the sidecar beside its `.part`, which cannot
/// go stale while it runs, and that account is left to own it. What has no `.part` and no
/// sidecar has nothing else at all: a queued row, and the Paused row a queued one comes
/// back as. Writing only the queued state is what made the queue survive one restart and
/// not the next.
fn only_record_of(view: &DownloadJob) -> bool {
    match view.state {
        DownloadState::Active => false,
        DownloadState::Queued => true,
        DownloadState::Paused => {
            let dest = Path::new(&view.path);
            !download::part_path(dest).exists() && !download::sidecar_path(dest).exists()
        }
        _ => view.state.finished(),
    }
}

/// What `downloads.json` is given: the history, and whatever it is the only account of.
fn persistable(jobs: &Mutex<Vec<Tracked>>) -> Vec<DownloadJob> {
    jobs.lock()
        .expect("downloads lock")
        .iter()
        .map(|job| job.view.clone())
        .filter(only_record_of)
        .collect()
}

const SIDECAR_SUFFIX: &str = ".part.json";

/// Every interrupted transfer the models directory is holding, keyed by where it was
/// headed. `.part` files are invisible to the catalog, which scans for `.gguf`, so
/// without this they accumulate unseen and their bytes are unreachable.
fn partials_in(models_dir: &Path) -> Vec<(PathBuf, download::Partial)> {
    let Ok(entries) = std::fs::read_dir(models_dir) else {
        return Vec::new();
    };

    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(stem) = name.strip_suffix(SIDECAR_SUFFIX) else {
            continue;
        };
        let dest = path.with_file_name(stem);
        if let Some(partial) = download::partial_at(&dest) {
            found.push((dest, partial));
        }
    }
    found.sort_by(|(a, _), (b, _)| a.cmp(b));
    found
}

/// Removes the bytes an abandoned transfer left behind. Both files or neither: a sidecar
/// without its `.part` is exactly the junk this exists to clear.
pub fn discard_files(dest: &Path) {
    let _ = std::fs::remove_file(download::part_path(dest));
    let _ = std::fs::remove_file(download::sidecar_path(dest));
}

/// Run once a file has landed in the models directory, so whoever owns the catalog can
/// take the new model into account. The manager knows a rename succeeded and nothing else.
pub type Landed = Arc<dyn Fn() + Send + Sync>;

/// What a started job runs. The engine is the only implementation the app ships — a URL
/// this manager admits can only name Hugging Face, so nothing else reaches the
/// bookkeeping a transfer is wrapped in.
pub type Engine =
    Arc<dyn Fn(&Spec, &Control, &dyn ProgressSink) -> Result<(), String> + Send + Sync>;

/// Where finished jobs are written so they outlive the process.
///
/// Only finished ones: an unfinished transfer is described by the sidecar beside its
/// `.part`, which is the only account of it that cannot go stale while it runs.
pub type Persist = Arc<dyn Fn(&[DownloadJob]) + Send + Sync>;

/// The job manager the commands drive: admission, the queue, and what settles.
///
/// Cloned into every transfer it starts, because a settled transfer promotes the next one
/// from its own thread, long after the caller that admitted it has gone.
#[derive(Clone)]
pub struct Downloads {
    jobs: Arc<Mutex<Vec<Tracked>>>,
    events: Events,
    landed: Landed,
    engine: Engine,
    persist: Persist,
    next: Arc<AtomicU64>,
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
            persist: Arc::new(|_| {}),
            next: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn persisting_with(mut self, persist: Persist) -> Self {
        self.persist = persist;
        self
    }

    pub fn snapshot(&self) -> Vec<DownloadJob> {
        views(&self.jobs.lock().expect("downloads lock"))
    }

    /// Seeds what a previous run wrote down: the history, and the queue it never got to.
    ///
    /// Nothing comes back Active. A transfer cannot survive the process that was running
    /// it, and one restored as Active would sit there forever holding the line. Whatever
    /// was mid-flight is on the disk instead, and `adopt` is what finds it.
    ///
    /// A queued row returns Paused rather than queued, and waits for a click: starting the
    /// app is not consent to fetch a URL that was read off disk.
    ///
    /// What it says about its progress is left alone. A row that never started has none to
    /// report, but one that was resumed into the queue is waiting over a `.part` — and
    /// since it holds that file's path, `adopt` takes it as already tracked and leaves it
    /// be. Zeroing it here is not a fresh start, it is the only account of those bytes
    /// claiming they are not there.
    ///
    /// Both the file name and the path are the app's to decide rather than the file's.
    /// Together they choose what a resume writes and what a discard deletes, and this
    /// file sits in a directory anything with write access can aim.
    pub fn restore(&self, history: Vec<DownloadJob>, models_dir: &Path) {
        let mut jobs = self.jobs.lock().expect("downloads lock");
        for mut view in history {
            if !plain_file_name(&view.file_name) {
                continue;
            }
            if view.state == DownloadState::Active {
                continue;
            }
            // Queued, or the Paused row a queued one came back as last time. Both are here
            // because nothing on disk describes them, and both wait for a click.
            if !view.state.finished() {
                if file_name_for(&view.url).is_err() {
                    continue;
                }
                view.state = DownloadState::Paused;
                view.resumable = true;
                view.phase = None;
                view.bytes_per_second = None;
                view.finished_secs = None;
            }

            self.reserve(&view.id);
            view.path = models_dir
                .join(&view.file_name)
                .to_string_lossy()
                .into_owned();
            jobs.push(Tracked {
                view,
                control: Arc::new(Control::default()),
                discarding: false,
                options: Options::default(),
            });
        }
    }

    /// Takes in the partials left in the models directory, so bytes fetched by a run that
    /// kept no history of them are still reachable. Untracked ones only — a `.part` being
    /// written right now already has a row pointing at it.
    pub fn adopt(&self, models_dir: &Path) -> Vec<DownloadJob> {
        let found = partials_in(models_dir);
        let added = {
            let mut jobs = self.jobs.lock().expect("downloads lock");
            let mut added = 0;
            for (dest, partial) in found {
                let path = dest.to_string_lossy();
                if jobs.iter().any(|job| job.view.path == path) {
                    continue;
                }
                let id = self.next_id();
                jobs.push(Tracked {
                    view: DownloadJob::adopted(&id, &dest, &partial),
                    control: Arc::new(Control::default()),
                    discarding: false,
                    options: Options::default(),
                });
                added += 1;
            }
            added
        };
        if added > 0 {
            self.announce();
        }
        self.snapshot()
    }

    fn next_id(&self) -> String {
        format!("dl-{}", self.next.fetch_add(1, Ordering::Relaxed))
    }

    /// Keeps the counter ahead of an id that came back from disk, so a new transfer cannot
    /// be handed one the restored history is already using.
    fn reserve(&self, id: &str) {
        let Some(used) = id.strip_prefix("dl-").and_then(|n| n.parse::<u64>().ok()) else {
            return;
        };
        self.next.fetch_max(used + 1, Ordering::Relaxed);
    }

    /// Takes a URL on. It runs now if the pipe is free and waits its turn if it is not.
    pub fn start(
        &self,
        url: &str,
        models_dir: &Path,
        options: &Options,
    ) -> Result<Vec<DownloadJob>, String> {
        let id = self.next_id();
        let control = Arc::new(Control::default());
        control.set_rate_limit(normalized_rate(options.rate_limit));

        let (dest, waiting, snapshot) = {
            let mut jobs = self.jobs.lock().expect("downloads lock");
            let dest = admit(url, models_dir, &views(&jobs))?;
            std::fs::create_dir_all(models_dir).map_err(|e| e.to_string())?;

            let waiting = jobs
                .iter()
                .any(|job| job.view.state == DownloadState::Active);
            let mut view = DownloadJob::begin(&id, url, &dest);
            if waiting {
                view.state = DownloadState::Queued;
                view.started_secs = None;
            }
            jobs.push(Tracked {
                view,
                control: control.clone(),
                discarding: false,
                options: options.clone(),
            });
            (dest, waiting, views(&jobs))
        };
        if waiting {
            (self.persist)(&persistable(&self.jobs));
        }
        self.announce();
        if !waiting {
            spawn(
                self.clone(),
                id,
                url.to_string(),
                dest,
                control,
                options.clone(),
            );
        }

        Ok(snapshot)
    }

    /// Puts a paused transfer back in line under its own id. It runs now if the pipe is
    /// free and waits its turn if it is not.
    ///
    /// The same job rather than a new one: the `.part` it stopped on is the same file, and
    /// two rows for one download read as a bug rather than as history.
    pub fn resume(
        &self,
        id: &str,
        models_dir: &Path,
        options: &Options,
    ) -> Result<Vec<DownloadJob>, String> {
        let control = Arc::new(Control::default());
        control.set_rate_limit(normalized_rate(options.rate_limit));

        let (url, dest, waiting, snapshot) = {
            let mut jobs = self.jobs.lock().expect("downloads lock");
            resumable(&views(&jobs), id)?;
            // `admit` is where a URL is checked, and a resume does not pass through it.
            // For an adopted row the URL came out of a sidecar on disk, so this is the
            // only thing between a planted file and a fetch from anywhere.
            let at = jobs
                .iter()
                .position(|job| job.view.id == id)
                .ok_or_else(|| format!("no download {id}"))?;
            file_name_for(&jobs[at].view.url)?;
            std::fs::create_dir_all(models_dir).map_err(|e| e.to_string())?;

            let waiting = jobs
                .iter()
                .any(|job| job.view.state == DownloadState::Active);

            // Joining the queue means joining the back of it. The list is the queue's
            // order, and a row resumed hours after it was first started would otherwise
            // sit at its old index and be served ahead of everything added since.
            let mut job = jobs.remove(at);

            // `completed` and `total` are left where they were, so the row opens on the
            // bytes it stopped at rather than at zero until the first report lands.
            job.view.error = None;
            job.view.phase = None;
            job.view.bytes_per_second = None;
            job.view.finished_secs = None;
            job.control = control.clone();
            job.discarding = false;
            job.options = options.clone();

            let url = job.view.url.clone();
            let dest = PathBuf::from(&job.view.path);

            if waiting {
                job.view.state = DownloadState::Queued;
                jobs.push(job);
            } else {
                job.view.state = DownloadState::Active;
                jobs.insert(at, job);
            }
            (url, dest, waiting, views(&jobs))
        };
        if waiting {
            (self.persist)(&persistable(&self.jobs));
        }
        self.announce();
        if !waiting {
            spawn(
                self.clone(),
                id.to_string(),
                url,
                dest,
                control,
                options.clone(),
            );
        }

        Ok(snapshot)
    }

    /// Throws the transfer away along with the bytes it fetched.
    ///
    /// A running transfer is marked and signalled; the files go on the settle path, once
    /// the engine has returned and stopped writing to them. One that has already stopped
    /// has no writer to wait for, so it goes now.
    pub fn discard(&self, id: &str) -> Result<Vec<DownloadJob>, String> {
        let snapshot = {
            let mut jobs = self.jobs.lock().expect("downloads lock");
            let job = jobs
                .iter_mut()
                .find(|job| job.view.id == id)
                .ok_or_else(|| format!("no download {id}"))?;

            if job.view.state == DownloadState::Active {
                job.discarding = true;
                job.control.cancel();
                return Ok(views(&jobs));
            }

            discard_files(&PathBuf::from(&job.view.path));
            jobs.retain(|job| job.view.id != id);
            views(&jobs)
        };
        (self.persist)(&persistable(&self.jobs));
        self.announce();
        Ok(snapshot)
    }

    /// Stops the transfer and keeps every byte it has fetched.
    pub fn pause(&self, id: &str) -> Result<Vec<DownloadJob>, String> {
        let jobs = self.jobs.lock().expect("downloads lock");
        let snapshot = views(&jobs);
        if cancellable(&snapshot, id)? {
            if let Some(job) = jobs.iter().find(|job| job.view.id == id) {
                job.control.cancel();
            }
        }
        Ok(snapshot)
    }

    fn announce(&self) {
        emit_state(&self.events, &self.jobs);
    }

    /// Applies a limit to whatever is running now as well as remembering it for what runs
    /// next. A limit only the next download honours is not the one the user was watching
    /// when they set it — and one that skipped the queue would be honoured by nothing the
    /// user can see, since a queued job carries the terms it was admitted on.
    pub fn set_rate_limit(&self, rate: Option<u64>) -> Vec<DownloadJob> {
        let mut jobs = self.jobs.lock().expect("downloads lock");
        for job in jobs.iter_mut() {
            match job.view.state {
                DownloadState::Active => job.control.set_rate_limit(normalized_rate(rate)),
                DownloadState::Queued => job.options.rate_limit = rate,
                _ => {}
            }
        }
        views(&jobs)
    }

    /// Drops the history. A paused or queued transfer is not history — one has bytes on
    /// disk waiting to be continued and the other has a place in a line, and clearing the
    /// finished rows must not throw either away.
    pub fn clear(&self) -> Vec<DownloadJob> {
        let snapshot = {
            let mut jobs = self.jobs.lock().expect("downloads lock");
            jobs.retain(|job| !job.view.state.finished());
            views(&jobs)
        };
        (self.persist)(&persistable(&self.jobs));
        emit_state(&self.events, &self.jobs);
        snapshot
    }
}

/// Hands a job to the engine on a thread of its own. The transfer blocks for as long as it
/// takes, which is hours, and hands the pipe to the queue when it is done.
fn spawn(
    rt: Downloads,
    id: String,
    url: String,
    dest: PathBuf,
    control: Arc<Control>,
    options: Options,
) {
    let spec = spec_for(&url, &dest, &options);
    let sink = Sink {
        id: id.clone(),
        jobs: rt.jobs.clone(),
        events: rt.events.clone(),
    };

    thread::spawn(move || {
        let outcome = (rt.engine)(&spec, &control, &sink);
        let complete = outcome.is_ok();
        finish(&rt.jobs, &id, outcome, &dest);
        // Ahead of the state event, so a screen that answers a finished transfer by
        // reading the catalog finds the file already in it.
        if complete {
            (rt.landed)();
        }
        (rt.persist)(&persistable(&rt.jobs));
        emit_state(&rt.events, &rt.jobs);
        advance(&rt);
    });
}

/// Starts the head of the queue, if there is one and the pipe is free.
///
/// Runs after `finish` has released the lock rather than inside it: starting a transfer
/// takes the same mutex, and a queue that promoted its next job from within the settle
/// path would deadlock on the first file that finished.
///
/// The loop is for the jobs it cannot start. A row admitted hours ago was checked against
/// a models directory that has changed since, and one whose file has landed in the meantime
/// is failed rather than allowed to overwrite it — then the next in line is considered.
fn advance(rt: &Downloads) {
    loop {
        let starting = {
            let mut jobs = rt.jobs.lock().expect("downloads lock");
            if jobs
                .iter()
                .any(|job| job.view.state == DownloadState::Active)
            {
                return;
            }
            let Some(at) = jobs
                .iter()
                .position(|job| job.view.state == DownloadState::Queued)
            else {
                return;
            };

            let dest = PathBuf::from(&jobs[at].view.path);
            let job = &mut jobs[at];
            if dest.exists() {
                job.view.state = DownloadState::Failed;
                job.view.error = Some(format!(
                    "{} is already in the models directory",
                    job.view.file_name
                ));
                job.view.finished_secs = now_secs();
                None
            } else {
                job.view.state = DownloadState::Active;
                job.view.started_secs = now_secs();
                job.control = Arc::new(Control::default());
                job.control
                    .set_rate_limit(normalized_rate(job.options.rate_limit));
                Some((
                    job.view.id.clone(),
                    job.view.url.clone(),
                    dest,
                    job.control.clone(),
                    job.options.clone(),
                ))
            }
        };

        (rt.persist)(&persistable(&rt.jobs));
        emit_state(&rt.events, &rt.jobs);

        let Some((id, url, dest, control, options)) = starting else {
            continue;
        };
        if let Some(models_dir) = dest.parent() {
            let _ = std::fs::create_dir_all(models_dir);
        }
        spawn(rt.clone(), id, url, dest, control, options);
        return;
    }
}

/// Records how a transfer ended, and throws away a discarded one entirely.
///
/// The deletion happens here rather than where the discard was asked for: `Control::cancel`
/// returns while the engine is still writing, and removing the `.part` from under a live
/// writer would race it. By the time this runs the engine has returned.
fn finish(jobs: &Mutex<Vec<Tracked>>, id: &str, outcome: Result<(), String>, dest: &Path) {
    let (state, error) = settle(outcome);
    let mut jobs = jobs.lock().expect("downloads lock");
    let Some(job) = jobs.iter_mut().find(|job| job.view.id == id) else {
        return;
    };

    if job.discarding {
        discard_files(dest);
        jobs.retain(|job| job.view.id != id);
        return;
    }

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
