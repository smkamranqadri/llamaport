use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

pub const CANCELLED: &str = "cancelled";

/// Twice a second: fast enough that a rate reads as live and a bar never looks stuck,
/// and slow enough that a 21 GB transfer emits thousands of reports rather than the
/// third of a million reads it is made of.
pub const DEFAULT_PROGRESS_EVERY: Duration = Duration::from_millis(500);

/// How much of a transfer a hard kill is allowed to cost, expressed as time rather than
/// bytes: whatever moved since the last flush is refetched on the next run.
pub const DEFAULT_FLUSH_EVERY: Duration = Duration::from_secs(2);

pub struct Spec {
    pub url: String,
    pub dest: PathBuf,
    pub segments: usize,
    pub stall_after: Duration,
    /// First delay before retrying a transient failure; doubles with each attempt.
    pub retry_backoff: Duration,
    /// Bytes per second across the whole transfer, not per segment.
    pub rate_limit: Option<u64>,
    pub verify: bool,
    /// Smallest gap between progress reports — a floor, not a period.
    pub progress_every: Duration,
    /// How often the resume sidecar is rewritten while the transfer runs.
    pub flush_every: Duration,
}

/// The engine reports through this rather than emitting Tauri events itself, so a
/// transfer can be observed by tests with no window to emit into.
pub trait ProgressSink: Send + Sync {
    fn report(&self, progress: Progress);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Resolving,
    Transferring,
    Verifying,
}

/// `completed` is what the current phase has done rather than a figure for the transfer
/// as a whole: verification re-reads the file from the start, and a bar that returns to
/// zero is the truth about what those minutes are being spent on.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub phase: Phase,
    pub completed: u64,
    /// Unknown until resolution has reported a size.
    pub total: Option<u64>,
    /// Absent on the first report of a phase, which has no earlier sample to difference
    /// against. A cumulative average would be a different, and much less useful, number.
    pub bytes_per_second: Option<f64>,
}

/// How often the transfer sampler wakes. This bounds how late a report can be, not how
/// many there are — `Spec::progress_every` decides that.
const PROGRESS_POLL: Duration = Duration::from_millis(50);

/// Throttles reports and measures the rate between the ones it lets through.
///
/// During a transfer this is driven from a thread of its own rather than from the segment
/// workers, so a slow sink cannot back up behind a socket read or stall the segments
/// behind a lock.
struct Meter<'a> {
    sink: &'a dyn ProgressSink,
    interval: Duration,
    last: Instant,
    previous: Option<(Phase, Instant, u64)>,
}

impl<'a> Meter<'a> {
    fn new(sink: &'a dyn ProgressSink, interval: Duration) -> Self {
        Self {
            sink,
            interval,
            last: Instant::now(),
            previous: None,
        }
    }

    fn tick(&mut self, phase: Phase, completed: u64, total: Option<u64>) {
        if self.last.elapsed() < self.interval {
            return;
        }
        self.mark(phase, completed, total);
    }

    fn mark(&mut self, phase: Phase, completed: u64, total: Option<u64>) {
        // Repeating a figure already reported would describe the gap since as a stall.
        if self
            .previous
            .is_some_and(|(before, _, bytes)| before == phase && bytes == completed)
        {
            return;
        }

        let now = Instant::now();
        let rate = self.previous.and_then(|(before, at, bytes)| {
            if before != phase || completed < bytes {
                return None;
            }
            let seconds = now.duration_since(at).as_secs_f64();
            if seconds <= 0.0 {
                return None;
            }
            Some((completed - bytes) as f64 / seconds)
        });

        self.last = now;
        self.previous = Some((phase, now, completed));
        self.sink.report(Progress {
            phase,
            completed,
            total,
            bytes_per_second: rate,
        });
    }
}

/// Shared with whoever started the transfer, so it can be stopped from elsewhere.
#[derive(Debug, Default)]
pub struct Control {
    cancelled: AtomicBool,
}

impl Control {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

struct Resolved {
    url: String,
    total: Option<u64>,
    /// `x-linked-etag`, which for an LFS file is the sha256 of the content.
    etag: Option<String>,
    accepts_ranges: bool,
}

/// What survives a process exit. Without it a resumed transfer knows only that a `.part`
/// file exists, not which of its bytes are real.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sidecar {
    source_url: String,
    total: u64,
    /// What the file was when these bytes were fetched. If upstream now reports
    /// something else, the partial belongs to a different file.
    etag: Option<String>,
    segments: Vec<SegmentState>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct SegmentState {
    start: u64,
    end: u64,
    /// Bytes already written within this segment, so it restarts at `start + completed`.
    completed: u64,
}

impl SegmentState {
    fn len(&self) -> u64 {
        self.end - self.start + 1
    }

    fn done(&self) -> bool {
        self.completed >= self.len()
    }
}

/// Resolution has the same hazard a segment read does — a peer that accepts the
/// connection and then says nothing — and no bound of its own, so it borrows the
/// segment's silence tolerance and adds an overall one for a chain that only dribbles.
fn agent(stall_after: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_read(stall_after)
        .timeout(stall_after * 4)
        .build()
}

/// A read timeout is what turns a silent socket into an error. Without one a segment that
/// has hung is indistinguishable from one that is merely slow, and the transfer waits on
/// it for as long as the peer keeps the connection open.
fn reading_agent(stall_after: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_read(stall_after)
        .build()
}

fn part_path(dest: &Path) -> PathBuf {
    let mut name = dest.as_os_str().to_os_string();
    name.push(".part");
    PathBuf::from(name)
}

fn sidecar_path(dest: &Path) -> PathBuf {
    let mut name = dest.as_os_str().to_os_string();
    name.push(".part.json");
    PathBuf::from(name)
}

/// A 403 means different things either side of a redirect: on the origin URL the repo is
/// gated and no retry will help, while on a CDN URL the signature has simply aged out.
enum ResolveError {
    Expired,
    Other(String),
}

impl From<String> for ResolveError {
    fn from(value: String) -> Self {
        ResolveError::Other(value)
    }
}

/// Re-runs resolution when the signature it was handed had already expired. Each attempt
/// issues a fresh one, so this converges rather than spinning.
fn resolve_signed(url: &str, stall_after: Duration) -> Result<Resolved, String> {
    for _ in 0..3 {
        match resolve(url, stall_after) {
            Ok(resolved) => return Ok(resolved),
            Err(ResolveError::Expired) => continue,
            Err(ResolveError::Other(e)) => return Err(e),
        }
    }
    Err("could not obtain a usable signed URL".into())
}

fn resolve(url: &str, stall_after: Duration) -> Result<Resolved, ResolveError> {
    let mut current = url.to_string();
    let mut total = None;
    let mut etag = None;
    let mut redirected = false;

    for _ in 0..10 {
        let response = match agent(stall_after).get(&current).call() {
            Ok(response) => response,
            Err(ureq::Error::Status(_, response)) => response,
            Err(e) => return Err(e.to_string().into()),
        };

        if total.is_none() {
            total = response
                .header("x-linked-size")
                .and_then(|value| value.parse().ok());
        }

        if etag.is_none() {
            etag = response
                .header("x-linked-etag")
                .map(|value| value.trim_matches('"').to_string());
        }

        if (300..400).contains(&response.status()) {
            let location = response
                .header("location")
                .ok_or_else(|| "redirect without a location header".to_string())?;
            current = location.to_string();
            redirected = true;
            continue;
        }

        if response.status() == 403 {
            if redirected {
                return Err(ResolveError::Expired);
            }
            return Err("access denied — this repository may be gated"
                .to_string()
                .into());
        }

        if response.status() >= 400 {
            return Err(format!("server returned {} for {current}", response.status()).into());
        }

        let accepts_ranges = response
            .header("accept-ranges")
            .is_some_and(|value| value.eq_ignore_ascii_case("bytes"));

        if total.is_none() {
            total = response
                .header("content-length")
                .and_then(|value| value.parse().ok());
        }

        return Ok(Resolved {
            url: current,
            total,
            etag,
            accepts_ranges,
        });
    }

    Err("too many redirects".to_string().into())
}

/// Contiguous inclusive ranges tiling `total`, with the remainder spread over the
/// leading segments rather than dumped on the last one.
fn plan_segments(total: u64, count: usize) -> Vec<(u64, u64)> {
    let count = (count as u64)
        .clamp(MIN_SEGMENTS, MAX_SEGMENTS)
        .min(total.max(1));
    let base = total / count;
    let remainder = total % count;

    let mut ranges = Vec::new();
    let mut start = 0;
    for index in 0..count {
        let mut len = base;
        if index < remainder {
            len += 1;
        }
        ranges.push((start, start + len - 1));
        start += len;
    }
    ranges
}

/// A segment vector worth restarting from: every range in order, none inverted, none
/// claiming more bytes than it holds, and together covering the file exactly. Anything
/// else describes some other file, and the `.part` it points at cannot be interpreted.
fn tiles(segments: &[SegmentState], total: u64) -> bool {
    let mut next = 0;
    for segment in segments {
        if segment.start != next || segment.end < segment.start || segment.end >= total {
            return false;
        }
        if segment.completed > segment.len() {
            return false;
        }
        next = segment.end + 1;
    }
    next == total
}

/// A sidecar is only trustworthy alongside the `.part` file it describes, and only when
/// it describes a file of the size the server is currently reporting. The `.part` is
/// preallocated to the full size on the first run, so anything shorter has been truncated
/// or replaced out of band and the ranges the sidecar calls complete are not on disk.
fn resume_state(dest: &Path, part: &Path, total: u64, etag: Option<&str>) -> Option<Sidecar> {
    if std::fs::metadata(part).ok()?.len() != total {
        return None;
    }
    let raw = std::fs::read_to_string(sidecar_path(dest)).ok()?;
    let sidecar: Sidecar = serde_json::from_str(&raw).ok()?;
    if sidecar.total != total {
        return None;
    }
    if !tiles(&sidecar.segments, total) {
        return None;
    }

    // An etag change means the upstream file was replaced. Every byte on disk belongs to
    // a different file, so the partial is worse than useless.
    if let (Some(recorded), Some(current)) = (sidecar.etag.as_deref(), etag) {
        if recorded != current {
            let _ = std::fs::remove_file(part);
            return None;
        }
    }

    Some(sidecar)
}

fn open_part(part: &Path, total: u64) -> Result<File, String> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(part)
        .map_err(|e| e.to_string())?;
    file.set_len(total).map_err(|e| e.to_string())?;
    Ok(file)
}

/// Written to a temporary path and renamed, so an interrupted write cannot leave a
/// truncated sidecar — which would be worse than none, having to be trusted on resume.
fn write_sidecar(dest: &Path, sidecar: &Sidecar) {
    let Ok(json) = serde_json::to_string(sidecar) else {
        return;
    };
    let path = sidecar_path(dest);
    let temporary = path.with_extension("writing");
    if std::fs::write(&temporary, json).is_ok() {
        let _ = std::fs::rename(&temporary, &path);
    }
}

/// A re-sign fails in two ways that have nothing in common: the round trip did not land,
/// which is the same class of accident as a dropped range read, or it landed and described
/// a different file, which no amount of retrying will undo.
enum Refresh {
    Unreachable(String),
    Changed(String),
}

/// The URL a segment is currently fetching from, the origin it was signed from, and what
/// the file was when the ranges now in flight were planned.
struct Source {
    origin: String,
    stall_after: Duration,
    etag: Option<String>,
    total: u64,
    current: Mutex<String>,
}

impl Source {
    fn url(&self) -> String {
        self.current.lock().expect("url lock").clone()
    }

    /// Re-signs the URL, holding no lock while it does: re-resolution is a network round
    /// trip, and every other segment reads this URL between ranges.
    fn refresh(&self, stale: &str) -> Result<(), Refresh> {
        if self.url() != stale {
            return Ok(());
        }

        let fresh = resolve_signed(&self.origin, self.stall_after).map_err(Refresh::Unreachable)?;

        // Re-resolution is the only look at upstream after the transfer starts. If it now
        // describes another file, the ranges in flight are planned against bytes that no
        // longer exist and continuing would mix two files in one `.part`.
        if fresh.etag != self.etag || fresh.total != Some(self.total) {
            return Err(Refresh::Changed(
                "the file changed upstream while it was being downloaded, so the bytes \
                 already fetched cannot be completed"
                    .into(),
            ));
        }

        let mut current = self.current.lock().expect("url lock");
        if current.as_str() == stale {
            *current = fresh.url;
        }
        Ok(())
    }
}

const ATTEMPTS: u32 = 5;
const BUFFER: usize = 64 * 1024;
const MIN_SEGMENTS: u64 = 4;
const MAX_SEGMENTS: u64 = 8;
/// Long enough to outlast a throttling window, short enough that a cancel is not left
/// waiting on a number the server chose.
const RETRY_AFTER_CAP: Duration = Duration::from_secs(30);
/// The longest a rate limit parks a segment before it looks up. A cancel is only observed
/// between reads, so an unbroken sleep is time the transfer cannot be stopped in — and a
/// low enough limit makes one buffer's worth of budget hours long.
const CHARGE_SLICE: Duration = Duration::from_millis(250);

/// One budget for the whole transfer.
///
/// A limit applied per segment is not a limit at all: it multiplies by the segment
/// count, so a 10 MB/s cap becomes 40 MB/s across four connections.
struct Bucket {
    rate: u64,
    capacity: f64,
    tokens: Mutex<(f64, Instant)>,
}

impl Bucket {
    fn new(rate: u64) -> Self {
        let capacity = BUFFER as f64;
        Self {
            rate: rate.max(1),
            capacity,
            tokens: Mutex::new((capacity, Instant::now())),
        }
    }

    /// Charges for bytes already read, blocking until the budget covers them. Charging
    /// after the fact keeps the accounting exact; charging a buffer's worth up front
    /// would over-charge on every short read.
    fn charge(&self, bytes: u64, control: &Control) {
        let mut owed = bytes as f64;

        while owed > 0.0 && !control.cancelled() {
            let wait = {
                let mut tokens = self.tokens.lock().expect("bucket lock");
                let now = Instant::now();
                let (available, last) = *tokens;
                let refilled = (available
                    + now.duration_since(last).as_secs_f64() * self.rate as f64)
                    .min(self.capacity);

                let spend = refilled.min(owed);
                *tokens = (refilled - spend, now);
                owed -= spend;

                if owed <= 0.0 {
                    return;
                }
                Duration::from_secs_f64(owed.min(self.capacity) / self.rate as f64)
            };
            thread::sleep(wait.min(CHARGE_SLICE));
        }
    }
}

/// Why one attempt at a range ended. Retrying is only right for one of these: a fatal
/// cause hammers a wall, and an expired signature is not a failure at all.
enum Fetch {
    Cancelled,
    Expired,
    /// Carries the delay the server asked for, when it asked for one.
    Transient(String, Option<Duration>),
    Fatal(String),
}

/// The first-byte-pos of a `Content-Range: bytes 0-15999/64000`. A body that starts
/// somewhere other than where the range asked for lands at the wrong offset.
fn first_byte_pos(header: &str) -> Option<u64> {
    let (first, _) = header.trim().strip_prefix("bytes ")?.split_once('-')?;
    first.trim().parse().ok()
}

fn retry_after(response: &ureq::Response) -> Option<Duration> {
    let seconds: u64 = response.header("retry-after")?.trim().parse().ok()?;
    Some(Duration::from_secs(seconds).min(RETRY_AFTER_CAP))
}

/// Stops the sibling segments once one of them hits something no retry will fix, so the
/// rest of a 20 GB file is not transferred after the transfer has already failed.
///
/// Separate from `Control` because that one belongs to the caller: setting it would
/// report a failure as a cancellation.
#[derive(Default)]
struct Abort {
    raised: AtomicBool,
    cause: Mutex<Option<String>>,
}

impl Abort {
    fn raise(&self, cause: &str) {
        let mut held = self.cause.lock().expect("abort lock");
        if held.is_none() {
            *held = Some(cause.to_string());
        }
        self.raised.store(true, Ordering::Relaxed);
    }

    /// The cause the first failing segment recorded. Siblings report that rather than one
    /// of their own, so whichever error is joined first is still the one that happened.
    fn cause(&self) -> Option<String> {
        if !self.raised.load(Ordering::Relaxed) {
            return None;
        }
        self.cause.lock().expect("abort lock").clone()
    }
}

/// Everything a segment worker needs that is identical for every segment.
struct Transfer<'a> {
    source: &'a Source,
    states: &'a Mutex<Vec<SegmentState>>,
    /// Bytes on disk across every segment, readable without taking the segment lock.
    moved: &'a AtomicU64,
    file: &'a File,
    control: &'a Control,
    abort: &'a Abort,
    bucket: Option<&'a Bucket>,
    backoff: Duration,
    stall_after: Duration,
}

fn attempt_range(transfer: &Transfer, url: &str, index: usize) -> Result<(), Fetch> {
    let Transfer {
        states,
        moved,
        file,
        control,
        abort,
        bucket,
        stall_after,
        ..
    } = transfer;

    let segment = states.lock().expect("segment lock")[index];
    let start = segment.start + segment.completed;

    let response = match reading_agent(*stall_after)
        .get(url)
        .set("Range", &format!("bytes={start}-{}", segment.end))
        .call()
    {
        Ok(response) => response,
        Err(ureq::Error::Status(403, _)) => return Err(Fetch::Expired),
        // Throttling and a server-side timeout are the transfer being asked to wait,
        // not being turned away.
        Err(ureq::Error::Status(code, response)) if code == 408 || code == 429 => {
            return Err(Fetch::Transient(
                format!("HTTP {code}"),
                retry_after(&response),
            ))
        }
        Err(ureq::Error::Status(code, _)) if code >= 500 => {
            return Err(Fetch::Transient(format!("HTTP {code}"), None))
        }
        Err(ureq::Error::Status(code, _)) => return Err(Fetch::Fatal(format!("HTTP {code}"))),
        Err(ureq::Error::Transport(transport)) => {
            return Err(Fetch::Transient(transport.to_string(), None))
        }
    };

    // A 200 here is the whole file, not this segment's slice — a server is allowed to
    // ignore Range, and writing that body at this segment's offset would scribble over
    // every other segment. A 3xx arrives as an ordinary response too, redirects being off.
    if response.status() != 206 {
        return Err(Fetch::Fatal(format!(
            "the server answered {} instead of honouring the range request, so the file \
             cannot be fetched in segments",
            response.status()
        )));
    }

    let reported = response.header("content-range").and_then(first_byte_pos);
    if reported != Some(start) {
        return Err(Fetch::Fatal(format!(
            "asked for bytes {start}-{} and the server answered content-range {}",
            segment.end,
            response.header("content-range").unwrap_or("(absent)")
        )));
    }

    let limit = segment.end + 1;
    let mut reader = response.into_reader();
    let mut buffer = vec![0u8; BUFFER];
    let mut offset = start;

    loop {
        if control.cancelled() {
            return Err(Fetch::Cancelled);
        }
        if let Some(cause) = abort.cause() {
            return Err(Fetch::Fatal(cause));
        }

        // A connection that dies mid-body is the ordinary case on a multi-hour transfer.
        // The bytes already written stay written, and the retry resumes behind them.
        let read = match reader.read(&mut buffer) {
            Ok(read) => read,
            Err(e) => return Err(Fetch::Transient(e.to_string(), None)),
        };
        if read == 0 {
            break;
        }

        if offset + read as u64 > limit {
            return Err(Fetch::Fatal(format!(
                "the server sent more bytes than the range {start}-{} asked for",
                segment.end
            )));
        }

        file.write_all_at(&buffer[..read], offset)
            .map_err(|e| Fetch::Fatal(e.to_string()))?;
        offset += read as u64;
        states.lock().expect("segment lock")[index].completed += read as u64;
        moved.fetch_add(read as u64, Ordering::Relaxed);

        if let Some(bucket) = bucket {
            bucket.charge(read as u64, control);
        }
    }

    // A clean end of stream short of the range asked for is a truncated response, not a
    // finished segment. Reporting success here would rename a file with a hole in it.
    if !states.lock().expect("segment lock")[index].done() {
        return Err(Fetch::Transient(
            "the connection ended before the range was complete".into(),
            None,
        ));
    }

    Ok(())
}

fn fetch_range(transfer: &Transfer, index: usize) -> Result<(), String> {
    let mut attempt = 0;
    let mut refreshes = 0;

    loop {
        if transfer.control.cancelled() {
            return Err(CANCELLED.into());
        }
        if let Some(cause) = transfer.abort.cause() {
            return Err(cause);
        }
        let before = transfer.states.lock().expect("segment lock")[index];
        if before.done() {
            return Ok(());
        }

        let url = transfer.source.url();
        let (cause, asked) = match attempt_range(transfer, &url, index) {
            Ok(()) => return Ok(()),
            Err(Fetch::Cancelled) => return Err(CANCELLED.into()),
            Err(Fetch::Fatal(e)) => return Err(e),
            // Not a failure, so it does not spend an attempt — but it is still bounded,
            // because a signature that will not stick is a failure of another kind.
            Err(Fetch::Expired) => {
                refreshes += 1;
                if refreshes > ATTEMPTS {
                    return Err("the signed URL expired repeatedly".into());
                }
                match transfer.source.refresh(&url) {
                    Ok(()) => continue,
                    Err(Refresh::Changed(e)) => return Err(e),
                    Err(Refresh::Unreachable(e)) => (e, None),
                }
            }
            Err(Fetch::Transient(e, asked)) => (e, asked),
        };

        // The budget is for a segment that is stuck, not for one that keeps being
        // interrupted: bytes landed since the last failure are a recovery, and a
        // multi-hour transfer will collect more than five of those.
        let now = transfer.states.lock().expect("segment lock")[index];
        if now.completed > before.completed {
            attempt = 0;
        }
        attempt += 1;
        if attempt >= ATTEMPTS {
            return Err(format!(
                "segment {index} failed after {ATTEMPTS} attempts: {cause}"
            ));
        }
        let mut delay = transfer.backoff * 2u32.pow(attempt - 1);
        if let Some(after) = asked {
            delay = after;
        }
        if let Some(cause) = transfer.abort.cause() {
            return Err(cause);
        }
        thread::sleep(delay);
    }
}

fn segmented(
    spec: &Spec,
    resolved: &Resolved,
    part: &Path,
    total: u64,
    control: &Control,
    resume: Option<Sidecar>,
    meter: &mut Meter,
) -> Result<(), String> {
    let segments = match resume {
        Some(sidecar) => sidecar.segments,
        None => plan_segments(total, spec.segments)
            .into_iter()
            .map(|(start, end)| SegmentState {
                start,
                end,
                completed: 0,
            })
            .collect(),
    };

    let file = open_part(part, total)?;
    let moved = AtomicU64::new(segments.iter().map(|segment| segment.completed).sum());
    let states = Mutex::new(segments);

    // `open_part` has just restored the `.part` to its full length, which is the evidence
    // a length-mismatched sidecar was rejected on. Leaving that sidecar on disk until the
    // first flush would let a kill in the meantime resume from a plan already judged
    // untrustworthy, against a `.part` that no longer looks wrong.
    write_sidecar(
        &spec.dest,
        &snapshot(&spec.url, total, resolved.etag.as_deref(), &states),
    );

    let finished = AtomicBool::new(false);
    let abort = Abort::default();
    let source = Source {
        origin: spec.url.clone(),
        stall_after: spec.stall_after,
        etag: resolved.etag.clone(),
        total,
        current: Mutex::new(resolved.url.clone()),
    };
    let bucket = spec.rate_limit.map(Bucket::new);
    let transfer = Transfer {
        source: &source,
        states: &states,
        moved: &moved,
        file: &file,
        control,
        abort: &abort,
        bucket: bucket.as_ref(),
        backoff: spec.retry_backoff,
        stall_after: spec.stall_after,
    };

    // Before a byte moves, so a transfer resuming at 60% opens there rather than climbing
    // from zero once the first sample lands.
    meter.mark(
        Phase::Transferring,
        moved.load(Ordering::Relaxed),
        Some(total),
    );

    let outcome = thread::scope(|scope| {
        scope.spawn(|| {
            while !finished.load(Ordering::Relaxed) {
                thread::sleep(PROGRESS_POLL);
                meter.tick(
                    Phase::Transferring,
                    moved.load(Ordering::Relaxed),
                    Some(total),
                );
            }
        });

        // Flushed while the transfer runs, not only at the end: a hard kill is exactly
        // the case the sidecar exists for, and it never gets to run cleanup.
        scope.spawn(|| {
            let mut last = Instant::now();
            while !finished.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(100));
                if last.elapsed() < spec.flush_every {
                    continue;
                }
                last = Instant::now();
                write_sidecar(
                    &spec.dest,
                    &snapshot(&spec.url, total, resolved.etag.as_deref(), &states),
                );
            }
        });

        let transfer = &transfer;

        let pending: Vec<usize> = {
            let segments = transfer.states.lock().expect("segment lock");
            (0..segments.len())
                .filter(|index| !segments[*index].done())
                .collect()
        };

        let mut workers = Vec::new();
        for index in pending {
            workers.push(scope.spawn(move || {
                let outcome = fetch_range(transfer, index);
                if let Err(cause) = &outcome {
                    if cause.as_str() != CANCELLED {
                        transfer.abort.raise(cause);
                    }
                }
                outcome
            }));
        }

        let mut outcome = Ok(());
        for worker in workers {
            let result = match worker.join() {
                Ok(result) => result,
                Err(_) => Err("a segment thread panicked".to_string()),
            };
            if outcome.is_ok() {
                outcome = result;
            }
        }

        finished.store(true, Ordering::Relaxed);
        outcome
    });

    write_sidecar(
        &spec.dest,
        &snapshot(&spec.url, total, resolved.etag.as_deref(), &states),
    );
    outcome?;

    // Nothing above proves the file is whole: a run with no pending segments spawns no
    // workers, so success here would mean success without a byte having been checked.
    if !states
        .lock()
        .expect("segment lock")
        .iter()
        .all(SegmentState::done)
    {
        return Err("the transfer ended with segments still incomplete".into());
    }
    Ok(())
}

fn snapshot(
    url: &str,
    total: u64,
    etag: Option<&str>,
    states: &Mutex<Vec<SegmentState>>,
) -> Sidecar {
    Sidecar {
        source_url: url.to_string(),
        total,
        etag: etag.map(str::to_string),
        segments: states.lock().expect("segment lock").clone(),
    }
}

/// Streams the file rather than reading it in: these are 13-21 GB.
///
/// Reported as it goes, because hashing one of them takes a minute or two and a screen
/// that shows nothing for that long is indistinguishable from one that has hung.
fn digest_of(
    path: &Path,
    total: u64,
    control: &Control,
    meter: &mut Meter,
) -> Result<String, String> {
    use sha2::{Digest, Sha256};

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; BUFFER];
    let mut hashed = 0u64;

    meter.mark(Phase::Verifying, 0, Some(total));

    loop {
        if control.cancelled() {
            return Err(CANCELLED.into());
        }
        let read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        hashed += read as u64;
        meter.tick(Phase::Verifying, hashed, Some(total));
    }

    meter.mark(Phase::Verifying, hashed, Some(total));
    Ok(format!("{:x}", hasher.finalize()))
}

/// Only an LFS etag is a content digest. Anything else is an opaque validator, and
/// comparing a hash against it would fail every time.
fn is_sha256(etag: &str) -> bool {
    etag.len() == 64 && etag.chars().all(|c| c.is_ascii_hexdigit())
}

/// Refuses before a byte moves rather than after 20 GB of one.
fn check_room(dest: &Path, total: u64, resume: Option<&Sidecar>) -> Result<(), String> {
    let directory = dest.parent().unwrap_or(Path::new("."));
    let Some((available, _)) = crate::catalog::disk_space(directory) else {
        return Ok(());
    };

    // Bytes already recorded are room the transfer does not need again. Recorded, not
    // the `.part`'s length: it is preallocated sparse, so its length is the full total
    // from the first run onward and measures nothing.
    let already: u64 = resume
        .map(|sidecar| sidecar.segments.iter().map(|s| s.completed).sum())
        .unwrap_or(0);
    let needed = total.saturating_sub(already);

    if available < needed {
        return Err(format!(
            "not enough free space — {} needs {needed} bytes and {available} are available",
            dest.file_name().unwrap_or_default().to_string_lossy()
        ));
    }
    Ok(())
}

pub fn download(spec: &Spec, control: &Control, progress: &dyn ProgressSink) -> Result<(), String> {
    let mut meter = Meter::new(progress, spec.progress_every);
    meter.mark(Phase::Resolving, 0, None);

    let resolved = resolve_signed(&spec.url, spec.stall_after)?;
    let part = part_path(&spec.dest);

    // Without ranges there is no resume, and an unresumable 20 GB transfer is a trap.
    if !resolved.accepts_ranges {
        return Err(
            "this server does not support range requests, so the download could \
                    not be resumed if it were interrupted"
                .into(),
        );
    }

    // A size is the same requirement wearing another hat: without one there are no
    // ranges to plan, and no way to tell a finished transfer from a truncated one.
    let Some(total) = resolved.total.filter(|total| *total > 0) else {
        return Err(
            "this server did not report the file size, so the download could not be \
                    resumed if it were interrupted"
                .into(),
        );
    };

    let resume = resume_state(&spec.dest, &part, total, resolved.etag.as_deref());
    check_room(&spec.dest, total, resume.as_ref())?;
    segmented(spec, &resolved, &part, total, control, resume, &mut meter)?;
    meter.mark(Phase::Transferring, total, Some(total));

    if spec.verify {
        if let Some(expected) = resolved.etag.as_deref().filter(|e| is_sha256(e)) {
            let actual = digest_of(&part, total, control, &mut meter)?;
            if actual != expected {
                // Discard rather than keep: a corrupt partial that looks resumable is how
                // a bad file ends up renamed into the models directory on the next run.
                let _ = std::fs::remove_file(&part);
                let _ = std::fs::remove_file(sidecar_path(&spec.dest));
                return Err(format!(
                    "sha256 digest mismatch — expected {expected}, got {actual}"
                ));
            }
        }
    }

    // Verification is a minute or two on these files, and a cancel raised anywhere in it
    // must not be answered by putting the file in the models directory anyway.
    if control.cancelled() {
        return Err(CANCELLED.into());
    }

    std::fs::rename(&part, &spec.dest).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(sidecar_path(&spec.dest));
    Ok(())
}
