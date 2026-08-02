//! Drives the download engine against a stand-in HTTP server.
//!
//! Hand-rolled rather than a framework: the cases worth testing are failure shapes —
//! an expiring signature, a stalled segment, a changed etag — which no real server can
//! be asked to produce on demand.

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use llamaport_lib::download::{self, Phase, Progress, ProgressSink, Spec};

/// For the cases that are about bytes rather than about what was reported.
struct Silent;

impl ProgressSink for Silent {
    fn report(&self, _: Progress) {}
}

#[derive(Default)]
struct Recorder {
    reports: Mutex<Vec<Progress>>,
}

impl ProgressSink for Recorder {
    fn report(&self, progress: Progress) {
        self.reports.lock().expect("reports lock").push(progress);
    }
}

impl Recorder {
    fn all(&self) -> Vec<Progress> {
        self.reports.lock().expect("reports lock").clone()
    }

    fn phase(&self, phase: Phase) -> Vec<Progress> {
        self.all()
            .into_iter()
            .filter(|report| report.phase == phase)
            .collect()
    }
}

/// Deterministic bytes, so a wrongly ordered or duplicated segment shows up as a
/// mismatch at a known offset rather than as plausible-looking noise.
fn body_of(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

fn scratch(name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let unique = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "llama-hub-dl-{}-{unique}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

struct Request {
    path: String,
    range: Option<(u64, u64)>,
}

fn read_request(stream: &TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);

    let mut start = String::new();
    if reader.read_line(&mut start).ok()? == 0 {
        return None;
    }
    let path = start.split_whitespace().nth(1)?.to_string();

    let mut range = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some(value) = line.trim().strip_prefix("Range: bytes=") {
            let (from, to) = value.split_once('-')?;
            range = Some((from.parse().ok()?, to.parse().ok()?));
        }
    }

    Some(Request { path, range })
}

struct Fake {
    port: u16,
    ranges: Arc<Mutex<Vec<(u64, u64)>>>,
    resolves: Arc<AtomicU32>,
    expired: Arc<AtomicU32>,
    ranged_bytes: Arc<AtomicU64>,
}

impl Fake {
    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/model.gguf", self.port)
    }

    /// Every range the server was asked for, in arrival order.
    fn ranges(&self) -> Vec<(u64, u64)> {
        self.ranges.lock().expect("ranges lock").clone()
    }

    /// How many times the redirect was followed — one per signature issued.
    fn resolves(&self) -> u32 {
        self.resolves.load(Ordering::Relaxed)
    }

    /// Ranged requests turned away because the signature they carried had expired.
    fn expired(&self) -> u32 {
        self.expired.load(Ordering::Relaxed)
    }

    /// Body bytes written in answer to ranged requests — what the transfer actually cost.
    fn ranged_bytes(&self) -> u64 {
        self.ranged_bytes.load(Ordering::Relaxed)
    }
}

/// The signature the CDN URL carries, as issued by the redirect that produced it.
fn signature(path: &str) -> u32 {
    path.rsplit("sig=")
        .next()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

#[derive(Clone, Copy, Default)]
struct Opts {
    chunk_delay: Duration,
    /// Rejects ranged requests carrying the first signature the redirect issued,
    /// standing in for a CDN signature that expires part way through a long transfer.
    expiring: bool,
    /// Accepts this many re-resolutions and hangs up without answering, leaving the very
    /// first resolution alone so the transfer gets under way before anything goes wrong.
    /// A connection reset during a routine mid-transfer re-sign.
    dropped_resigns: u32,
    /// Answers this many ranged requests with a 503 before serving normally.
    flaky: u32,
    /// Answers this many ranged requests with a 429 before serving normally.
    throttled: u32,
    /// Serves this many ranged requests as a prefix of the range and then hangs up,
    /// which is a reset mid-body: real progress, and an incomplete segment.
    truncating: u32,
    /// Answers ranged requests with a 404, leaving resolution intact.
    missing: bool,
    /// Answers only the range that starts at byte 0 with a 404 and serves every other
    /// range normally, so one segment hits a wall while its siblings are mid-body.
    missing_first_range: bool,
    /// Answers every ranged request with headers and then goes quiet without sending a
    /// byte, which parks a fresh transfer between planning and its first progress.
    stall_always: bool,
    /// Advertises range support and then answers a ranged request with the whole file,
    /// which a server is free to do and a caching proxy does.
    ignores_ranges: bool,
    /// Reports no size at all: no `x-linked-size`, no `Content-Length`.
    no_size: bool,
    /// Sends half of the first ranged request's bytes and then goes quiet without
    /// closing, which is how a transfer ends up parked at 97% forever.
    stall_first: bool,
    /// Advertises the digest of the real body but serves altered bytes.
    corrupt: bool,
    /// Claims a size no disk could hold.
    huge: bool,
    /// Serves the whole body and never advertises `accept-ranges`.
    no_ranges: bool,
    /// Advertises a different etag on every resolution after the first, standing in for
    /// the file behind a `main` ref being replaced part way through a transfer.
    swapped_upstream: bool,
    /// Serves the first so many bytes of every range at `chunk_delay` and everything
    /// after it at the second delay: a line that collapses part way through a transfer.
    slows_after: Option<(u64, Duration)>,
}

/// The etag a swapped-upstream server reports once the file behind the ref has changed.
const REPLACEMENT_ETAG: &str = "beef000000000000000000000000000000000000000000000000000000000000";

const PETABYTE: u64 = 1_000_000_000_000_000;

fn start(body: Vec<u8>) -> Fake {
    start_with(body, Opts::default())
}

fn start_paced(body: Vec<u8>, chunk_delay: Duration) -> Fake {
    start_with(
        body,
        Opts {
            chunk_delay,
            ..Opts::default()
        },
    )
}

/// Serves `/model.gguf` as a redirect to `/cdn/model.gguf`, mirroring Hugging Face:
/// the true size and digest ride on the redirect, and only the CDN URL serves bytes.
///
/// `chunk_delay` paces the body so a transfer can be caught in flight.
fn start_with(body: Vec<u8>, opts: Opts) -> Fake {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let ranges = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&ranges);
    let resolves = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&resolves);
    let expired = Arc::new(AtomicU32::new(0));
    let expiries = Arc::clone(&expired);
    let served = Arc::new(AtomicU32::new(0));
    let stalled = Arc::new(AtomicU32::new(0));
    let throttles = Arc::new(AtomicU32::new(0));
    let truncations = Arc::new(AtomicU32::new(0));
    let dropped = Arc::new(AtomicU32::new(0));
    let ranged_bytes = Arc::new(AtomicU64::new(0));
    let meter = Arc::clone(&ranged_bytes);

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let advertised = body.clone();
            let mut body = body.clone();
            if opts.corrupt {
                body[9] ^= 0xff;
            }
            let recorder = Arc::clone(&recorder);
            let counter = Arc::clone(&counter);
            let expiries = Arc::clone(&expiries);
            let failures = Arc::clone(&served);
            let stalls = Arc::clone(&stalled);
            let throttles = Arc::clone(&throttles);
            let truncations = Arc::clone(&truncations);
            let dropped = Arc::clone(&dropped);
            let meter = Arc::clone(&meter);
            let chunk_delay = opts.chunk_delay;
            thread::spawn(move || {
                let Some(request) = read_request(&stream) else {
                    return;
                };
                if let Some(range) = request.range {
                    recorder.lock().expect("ranges lock").push(range);
                }

                if !request.path.contains("/cdn/") {
                    // Each redirect issues a distinct signature, so a refreshed URL is
                    // distinguishable from the stale one it replaced.
                    let signature = counter.fetch_add(1, Ordering::Relaxed) + 1;
                    if signature > 1
                        && dropped.fetch_add(1, Ordering::Relaxed) < opts.dropped_resigns
                    {
                        return;
                    }
                    let claimed = match opts.huge {
                        true => PETABYTE,
                        false => body.len() as u64,
                    };
                    let mut size = format!("x-linked-size: {claimed}\r\n");
                    if opts.no_size {
                        size.clear();
                    }
                    let mut etag = digest_of(&advertised);
                    if opts.swapped_upstream && signature > 1 {
                        etag = REPLACEMENT_ETAG.to_string();
                    }
                    let _ = write!(
                        stream,
                        "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{port}/cdn/model.gguf?sig={signature}\r\n{size}x-linked-etag: \"{etag}\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    return;
                }

                if opts.expiring && request.range.is_some() && signature(&request.path) == 1 {
                    expiries.fetch_add(1, Ordering::Relaxed);
                    let _ = write!(
                        stream,
                        "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    return;
                }

                if opts.missing && request.range.is_some() {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    return;
                }

                if opts.missing_first_range && matches!(request.range, Some((0, _))) {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    return;
                }

                if request.range.is_some() && failures.fetch_add(1, Ordering::Relaxed) < opts.flaky
                {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    return;
                }

                if request.range.is_some()
                    && throttles.fetch_add(1, Ordering::Relaxed) < opts.throttled
                {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    return;
                }

                if opts.ignores_ranges && request.range.is_some() {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(&body);
                    let _ = stream.flush();
                    return;
                }

                if opts.no_size && request.range.is_none() {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n"
                    );
                    let _ = stream.write_all(&body);
                    let _ = stream.flush();
                    return;
                }

                if opts.no_ranges {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(&body);
                    let _ = stream.flush();
                    return;
                }

                match request.range {
                    Some((from, to)) => {
                        let slice = &body[from as usize..=(to as usize).min(body.len() - 1)];
                        let _ = write!(
                            stream,
                            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {from}-{to}/{}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                            body.len(),
                            slice.len()
                        );
                        if opts.stall_always {
                            let _ = stream.flush();
                            thread::sleep(Duration::from_secs(30));
                            return;
                        }
                        let stalling =
                            opts.stall_first && stalls.fetch_add(1, Ordering::Relaxed) == 0;
                        let truncating =
                            truncations.fetch_add(1, Ordering::Relaxed) < opts.truncating;

                        for (sent, chunk) in slice.chunks(4096).enumerate() {
                            if stalling && sent * 4096 >= slice.len() / 2 {
                                thread::sleep(Duration::from_secs(30));
                                return;
                            }
                            // A reset mid-body: the bytes already sent are real, and the
                            // connection dies before the range is finished.
                            if truncating && sent > 0 {
                                return;
                            }
                            if stream.write_all(chunk).is_err() {
                                return;
                            }
                            meter.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                            let _ = stream.flush();
                            let mut delay = chunk_delay;
                            if let Some((fast, slower)) = opts.slows_after {
                                if (sent * 4096) as u64 >= fast {
                                    delay = slower;
                                }
                            }
                            if !delay.is_zero() {
                                thread::sleep(delay);
                            }
                        }
                    }
                    None => {
                        let _ = write!(
                            stream,
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(&body);
                    }
                }
                let _ = stream.flush();
            });
        }
    });

    Fake {
        port,
        ranges,
        resolves,
        expired,
        ranged_bytes,
    }
}

fn digest_of(body: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(body);
    format!("{:x}", hasher.finalize())
}

fn spec(url: String, dest: PathBuf) -> Spec {
    Spec {
        url,
        dest,
        segments: 4,
        stall_after: Duration::from_millis(400),
        retry_backoff: Duration::from_millis(10),
        verify: false,
        progress_every: download::DEFAULT_PROGRESS_EVERY,
        flush_every: download::DEFAULT_FLUSH_EVERY,
    }
}

#[test]
fn downloads_a_redirected_file_intact() {
    let body = body_of(64_000);
    let server = start(body.clone());
    let dir = scratch("intact");
    let dest = dir.join("model.gguf");

    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("download");

    let mut written = Vec::new();
    std::fs::File::open(&dest)
        .expect("open result")
        .read_to_end(&mut written)
        .expect("read result");

    assert_eq!(written.len(), body.len());
    assert_eq!(written, body);
}

#[test]
fn splits_the_transfer_into_ranged_segments() {
    let body = body_of(64_000);
    let server = start(body.clone());
    let dir = scratch("segments");
    let dest = dir.join("model.gguf");

    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("download");

    let mut ranges = server.ranges();
    ranges.sort();

    assert_eq!(ranges.len(), 4, "expected one request per segment");
    assert_eq!(ranges[0].0, 0);
    assert_eq!(ranges[3].1, body.len() as u64 - 1);
    for pair in ranges.windows(2) {
        assert_eq!(
            pair[1].0,
            pair[0].1 + 1,
            "segments must tile the file without gaps or overlap"
        );
    }

    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

#[test]
fn resumes_from_the_sidecar_without_refetching_completed_bytes() {
    let body = body_of(64_000);
    let server = start(body.clone());
    let dir = scratch("resume");
    let dest = dir.join("model.gguf");
    let part = dir.join("model.gguf.part");

    // A transfer interrupted after its first segment finished. The part file already
    // holds those bytes; the sidecar is what says so after the process died.
    let mut partial = vec![0u8; 64_000];
    partial[..16_000].copy_from_slice(&body[..16_000]);
    std::fs::write(&part, &partial).expect("seed part");
    std::fs::write(
        dir.join("model.gguf.part.json"),
        format!(
            r#"{{"sourceUrl":"{}","total":64000,"etag":"{}","segments":[
                {{"start":0,"end":15999,"completed":16000}},
                {{"start":16000,"end":31999,"completed":0}},
                {{"start":32000,"end":47999,"completed":0}},
                {{"start":48000,"end":63999,"completed":0}}]}}"#,
            server.url(),
            digest_of(&body)
        ),
    )
    .expect("seed sidecar");

    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("download");

    let ranges = server.ranges();
    assert!(
        ranges.iter().all(|(start, _)| *start >= 16_000),
        "completed bytes must not be refetched, saw {ranges:?}"
    );
    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// Routine on a multi-hour transfer, and must be invisible: the signature dies, not the
/// download.
#[test]
fn re_resolves_an_expired_signature_instead_of_failing() {
    let body = body_of(64_000);
    let server = start_with(
        body.clone(),
        Opts {
            expiring: true,
            ..Opts::default()
        },
    );
    let dir = scratch("expiry");
    let dest = dir.join("model.gguf");

    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("an expired signature must not fail the transfer");

    // The 403 has to land on a ranged request, not on resolution: resolution retries on
    // its own, so a transfer that never reaches a segment proves nothing about refresh.
    assert!(
        server.expired() > 0,
        "no ranged request was answered 403, so the mid-transfer path was never taken"
    );
    assert!(
        server.resolves() >= 2,
        "a 403 on the CDN must trigger a re-resolve, saw {} resolves",
        server.resolves()
    );
    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// Re-signing is itself a network round trip, so it fails the way every other round trip
/// does. Parking the transfer on that turns an event the spec calls invisible into a
/// failure the user has to resume from by hand.
#[test]
fn a_lost_re_sign_is_retried_instead_of_parking_the_transfer() {
    let body = body_of(64_000);
    let server = start_with(
        body.clone(),
        Opts {
            expiring: true,
            dropped_resigns: 1,
            ..Opts::default()
        },
    );
    let dir = scratch("resign-reset");
    let dest = dir.join("model.gguf");

    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("a re-sign that did not land must be retried, not fail the transfer");

    assert!(
        server.expired() > 0,
        "no ranged request was answered 403, so no re-sign was ever needed"
    );
    // The first resolution, the one that was dropped, and the one that replaced it.
    assert!(
        server.resolves() >= 3,
        "the dropped re-sign should have been tried again, saw {} resolves",
        server.resolves()
    );
    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// Re-resolution is the only look at upstream after the transfer starts. If the file
/// behind the ref was replaced in the meantime, carrying on writes bytes from the new
/// file into offsets planned for the old one.
#[test]
fn a_file_replaced_mid_transfer_fails_instead_of_mixing_two_files() {
    let body = body_of(64_000);
    let server = start_with(
        body,
        Opts {
            expiring: true,
            swapped_upstream: true,
            ..Opts::default()
        },
    );
    let dir = scratch("swapped");
    let dest = dir.join("model.gguf");

    let error = download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect_err("a file replaced mid-transfer must not be reported as downloaded");

    assert!(
        server.expired() > 0,
        "no ranged request was answered 403, so the re-resolution path was never taken"
    );
    assert!(
        error.to_lowercase().contains("changed"),
        "the error should say the file changed upstream, got {error}"
    );
    assert!(!dest.exists(), "nothing may be renamed into place");
}

#[test]
fn retries_a_transient_failure_instead_of_abandoning_the_transfer() {
    let body = body_of(64_000);
    let server = start_with(
        body.clone(),
        Opts {
            flaky: 4,
            ..Opts::default()
        },
    );
    let dir = scratch("transient");
    let dest = dir.join("model.gguf");

    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("a 503 is transient and must be retried");

    assert!(
        server.ranges().len() > 4,
        "each rejected segment should have been retried, saw {:?}",
        server.ranges()
    );
    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// These files are 13-21 GB. Finding out there is no room for one after transferring
/// 20 GB of it is the expensive way to learn it.
#[test]
fn refuses_a_file_the_disk_cannot_hold() {
    let body = body_of(64_000);
    let server = start_with(
        body,
        Opts {
            huge: true,
            ..Opts::default()
        },
    );
    let dir = scratch("disk");
    let dest = dir.join("model.gguf");

    let error = download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect_err("a file larger than the disk must be refused");

    let lowered = error.to_lowercase();
    assert!(
        lowered.contains("space") || lowered.contains("disk"),
        "the error should say the disk is the problem, got {error}"
    );
    assert!(
        server.ranges().is_empty(),
        "nothing should be transferred before the check, saw {:?}",
        server.ranges()
    );
    assert!(!dir.join("model.gguf.part").exists());
}

/// Without ranges there is no resume, and a 20 GB transfer that cannot resume is a trap
/// rather than a convenience.
#[test]
fn refuses_a_server_that_will_not_serve_ranges() {
    let body = body_of(64_000);
    let server = start_with(
        body,
        Opts {
            no_ranges: true,
            ..Opts::default()
        },
    );
    let dir = scratch("noranges");
    let dest = dir.join("model.gguf");

    let error = download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect_err("a server without range support must be refused");

    let lowered = error.to_lowercase();
    assert!(
        lowered.contains("range") || lowered.contains("resume"),
        "the error should name range support, got {error}"
    );
    assert!(!dest.exists());
}

#[test]
fn verification_accepts_an_intact_download() {
    let body = body_of(64_000);
    let server = start(body.clone());
    let dir = scratch("verify-ok");
    let dest = dir.join("model.gguf");

    let mut checked = spec(server.url(), dest.clone());
    checked.verify = true;

    download::download(&checked, &download::Control::default(), &Silent)
        .expect("an intact download must pass verification");

    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// A corrupt 20 GB file renamed into the models directory looks exactly like a good one
/// until something tries to load it.
#[test]
fn verification_rejects_bytes_that_do_not_match_the_digest() {
    let body = body_of(64_000);
    let server = start_with(
        body,
        Opts {
            corrupt: true,
            ..Opts::default()
        },
    );
    let dir = scratch("verify-bad");
    let dest = dir.join("model.gguf");

    let mut checked = spec(server.url(), dest.clone());
    checked.verify = true;

    let error = download::download(&checked, &download::Control::default(), &Silent)
        .expect_err("a digest mismatch must fail the download");

    assert!(
        error.to_lowercase().contains("digest") || error.to_lowercase().contains("sha256"),
        "the error should say what failed, got {error}"
    );
    assert!(
        !dest.exists(),
        "a file that failed its digest must not be kept"
    );
    assert!(
        !dir.join("model.gguf.part").exists(),
        "the corrupt partial should be discarded, not left to be resumed"
    );
}

/// Cancels once the transfer reports having reached a point, so a cancel can be placed
/// inside a window the clock cannot be relied on to land in.
struct CancelAt {
    phase: Phase,
    completed: u64,
    control: Arc<download::Control>,
    seen: Recorder,
}

impl ProgressSink for CancelAt {
    fn report(&self, progress: Progress) {
        self.seen.report(progress);
        if progress.phase == self.phase && progress.completed >= self.completed {
            self.control.cancel();
        }
    }
}

/// Hashing one of these files takes a minute or two, and the cancel button is live for
/// every second of it. A cancel spent there that is answered by renaming the file into
/// the models directory anyway puts the model the user cancelled into their library.
#[test]
fn cancelling_during_verification_keeps_the_file_out_of_the_models_directory() {
    let body = body_of(400_000);
    let server = start(body);
    let dir = scratch("cancel-verify");
    let dest = dir.join("model.gguf");

    let control = Arc::new(download::Control::default());
    let mut checked = spec(server.url(), dest.clone());
    checked.verify = true;
    checked.progress_every = Duration::ZERO;

    let watcher = CancelAt {
        phase: Phase::Verifying,
        completed: 0,
        control: Arc::clone(&control),
        seen: Recorder::default(),
    };

    let error = download::download(&checked, &control, &watcher)
        .expect_err("a cancel during verification must not report success");

    assert_eq!(
        error,
        download::CANCELLED,
        "the outcome has to read as a cancellation rather than as a failure"
    );
    assert!(!dest.exists(), "nothing may be renamed into place");
    assert!(
        dir.join("model.gguf.part").exists(),
        "the bytes already fetched must survive for the next run to resume from"
    );

    // Reporting every read, so hashing that carried on past the cancel shows up as more
    // than the one report the pass had already made when it was told to stop. On a 21 GB
    // file that is another minute or two spent on a download the user has ended.
    let verifying = watcher.seen.phase(Phase::Verifying);
    assert_eq!(
        verifying.len(),
        1,
        "the hashing pass ran on after the cancel, saw {verifying:?}"
    );
}

/// The last byte landing and the rename are not the same moment, and a cancel between
/// them is still a cancel.
#[test]
fn a_cancel_that_lands_before_the_rename_is_not_a_completed_download() {
    let body = body_of(400_000);
    let total = body.len() as u64;
    let server = start(body);
    let dir = scratch("cancel-rename");
    let dest = dir.join("model.gguf");

    let control = Arc::new(download::Control::default());
    let mut watched = spec(server.url(), dest.clone());
    watched.progress_every = Duration::ZERO;

    let error = download::download(
        &watched,
        &control,
        &CancelAt {
            phase: Phase::Transferring,
            completed: total,
            control: Arc::clone(&control),
            seen: Recorder::default(),
        },
    )
    .expect_err("a cancelled transfer must not report success");

    assert_eq!(error, download::CANCELLED);
    assert!(!dest.exists(), "nothing may be renamed into place");
}

/// An etag change means the upstream file was replaced, so the bytes already on disk
/// belong to a different file.
#[test]
fn a_changed_etag_discards_the_partial_and_starts_over() {
    let body = body_of(64_000);
    let server = start(body.clone());
    let dir = scratch("etag");
    let dest = dir.join("model.gguf");
    let part = dir.join("model.gguf.part");

    let mut stale = vec![0u8; 64_000];
    stale[..16_000].copy_from_slice(&body[..16_000]);
    std::fs::write(&part, &stale).expect("seed part");
    std::fs::write(
        dir.join("model.gguf.part.json"),
        format!(
            r#"{{"sourceUrl":"{}","total":64000,"etag":"0000000000000000000000000000000000000000000000000000000000000000","segments":[
                {{"start":0,"end":15999,"completed":16000}},
                {{"start":16000,"end":31999,"completed":0}},
                {{"start":32000,"end":47999,"completed":0}},
                {{"start":48000,"end":63999,"completed":0}}]}}"#,
            server.url()
        ),
    )
    .expect("seed sidecar");

    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("download");

    assert!(
        server.ranges().iter().any(|(start, _)| *start == 0),
        "the first segment must be refetched, not trusted, saw {:?}",
        server.ranges()
    );
    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// A budget handed to each segment separately is not a limit: it multiplies by the
/// segment count, so a 10 MB/s cap quietly becomes 40 MB/s.
#[test]
fn the_rate_limit_is_shared_across_segments() {
    let body = body_of(200_000);
    let server = start(body.clone());
    let dir = scratch("ratelimit");
    let dest = dir.join("model.gguf");

    let limited = spec(server.url(), dest.clone());
    let control = download::Control::default();
    control.set_rate_limit(Some(100_000));

    let started = Instant::now();
    download::download(&limited, &control, &Silent).expect("download");
    let elapsed = started.elapsed();

    // Each of the four segments is 50 KB — small enough to fit in an initial burst, so
    // per-segment budgets would let the whole transfer through almost immediately.
    assert!(
        elapsed >= Duration::from_millis(900),
        "200 KB at 100 KB/s finished in {elapsed:?}, so the limit was not shared"
    );
    // A guard against a hang rather than a measurement of the budget: the lower bound is
    // what tells a shared budget from a per-segment one, and this only has to end.
    assert!(
        elapsed < Duration::from_secs(60),
        "far too slow: {elapsed:?}"
    );
    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// A budget of a byte a second makes one buffer's worth of it hours long, and the flag a
/// cancel sets is only read between reads. Waiting the budget out in one piece would leave
/// the transfer running, the job Active, and the app refusing every download after it.
#[test]
fn a_cancel_reaches_a_transfer_waiting_on_its_rate_limit() {
    let body = body_of(300_000);
    let server = start(body);
    let dir = scratch("ratelimit-cancel");

    let mut crawling = spec(server.url(), dir.join("model.gguf"));
    crawling.progress_every = Duration::from_millis(10);

    let reports = Arc::new(Recorder::default());
    let watched = Arc::clone(&reports);
    let control = Arc::new(download::Control::default());
    control.set_rate_limit(Some(1));
    let running = Arc::clone(&control);
    let (finished, waiting) = mpsc::channel();
    thread::spawn(move || {
        let _ = finished.send(download::download(&crawling, &running, &*watched));
    });

    // The budget starts full at one buffer, and every charge past that is a sleep no
    // cancel can be answered during. Placed after it, so the cancel lands on one.
    let moved = || {
        reports
            .phase(Phase::Transferring)
            .last()
            .map_or(0, |report| report.completed)
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    while moved() < 64 * 1024 {
        assert!(
            Instant::now() < deadline,
            "the transfer never spent its initial budget, reached {}",
            moved()
        );
        thread::sleep(Duration::from_millis(5));
    }

    control.cancel();
    let error = waiting
        .recv_timeout(Duration::from_secs(5))
        .expect("the cancel never reached the transfer")
        .expect_err("a cancelled transfer must not report success");

    assert_eq!(error, download::CANCELLED);
}

/// A limit is something the user changes while watching the transfer it applies to, so it
/// has to reach one that is already running. The segments are parked inside a sleep when
/// the change arrives, which is the case that would otherwise only take effect on the next
/// download — or never, on a transfer with hours left.
#[test]
fn a_rate_limit_lifted_mid_transfer_reaches_the_transfer_it_is_lifted_on() {
    let body = body_of(300_000);
    let server = start(body.clone());
    let dir = scratch("ratelimit-live");
    let dest = dir.join("model.gguf");

    let mut crawling = spec(server.url(), dest.clone());
    crawling.progress_every = Duration::from_millis(10);

    let reports = Arc::new(Recorder::default());
    let watched = Arc::clone(&reports);
    let control = Arc::new(download::Control::default());
    control.set_rate_limit(Some(1));
    let running = Arc::clone(&control);
    let (finished, waiting) = mpsc::channel();
    thread::spawn(move || {
        let _ = finished.send(download::download(&crawling, &running, &*watched));
    });

    // The budget starts full at one buffer and is spent at a byte a second after that, so
    // waiting for it to run out is what puts every segment inside a sleep before the lift.
    let moved = || {
        reports
            .phase(Phase::Transferring)
            .last()
            .map_or(0, |report| report.completed)
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    while moved() < 64 * 1024 {
        assert!(
            Instant::now() < deadline,
            "the transfer never spent its initial budget, reached {}",
            moved()
        );
        thread::sleep(Duration::from_millis(5));
    }

    // The remaining 230 KB at a byte a second is three days of transfer. Finishing at all
    // is the assertion; the timeout is what makes it one.
    control.set_rate_limit(None);

    waiting
        .recv_timeout(Duration::from_secs(20))
        .expect("the lifted limit never reached the running transfer")
        .expect("download");
    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// Without this a transfer sits at 97% indefinitely: the socket is open, nothing is
/// wrong enough to raise an error, and no bytes are arriving.
#[test]
fn reissues_a_stalled_segment_instead_of_waiting_forever() {
    let body = body_of(64_000);
    let server = start_with(
        body.clone(),
        Opts {
            stall_first: true,
            ..Opts::default()
        },
    );
    let dir = scratch("stall");
    let dest = dir.join("model.gguf");

    let started = Instant::now();
    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("a stalled segment must be reissued, not waited on");
    let elapsed = started.elapsed();

    // The stand-in holds the socket open for 30s. Finishing only after it gives up is
    // not detection — the transfer has to notice the silence itself and act on it.
    assert!(
        elapsed < Duration::from_secs(15),
        "the stall should have been spotted near the {}ms threshold, took {elapsed:?}",
        spec(server.url(), dest.clone()).stall_after.as_millis()
    );
    assert!(
        server.ranges().len() > 4,
        "the stalled range should have been requested again, saw {:?}",
        server.ranges()
    );
    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// Retrying a file that is not there just hammers a wall.
#[test]
fn a_missing_file_stops_without_retrying() {
    let body = body_of(64_000);
    let server = start_with(
        body,
        Opts {
            missing: true,
            ..Opts::default()
        },
    );
    let dir = scratch("missing");
    let dest = dir.join("model.gguf");

    let error = download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect_err("a 404 must fail the transfer");

    assert!(
        error.contains("404") || error.to_lowercase().contains("not found"),
        "the error should name the status, got {error}"
    );
    // A 404 is answered by stopping, so no range is ever asked for twice. Not a count:
    // the first fatal stops the siblings, and one that had not yet issued its request
    // never asks for anything at all.
    let ranges = server.ranges();
    let distinct: HashSet<(u64, u64)> = ranges.iter().copied().collect();
    assert!(!ranges.is_empty(), "no range was requested at all");
    assert_eq!(
        distinct.len(),
        ranges.len(),
        "a 404 must not be retried, saw {ranges:?}"
    );
    assert!(!dest.exists());
}

#[test]
fn cancelling_leaves_a_sidecar_the_next_run_resumes_from() {
    // Large enough, and paced slowly enough, that the cancel lands mid-transfer with
    // room to spare rather than racing the last chunk.
    let body = body_of(400_000);
    let server = Arc::new(start_paced(body.clone(), Duration::from_millis(20)));
    let dir = scratch("cancel");
    let dest = dir.join("model.gguf");

    let control = Arc::new(download::Control::default());
    let trigger = Arc::clone(&control);
    let watcher = Arc::clone(&server);
    thread::spawn(move || {
        // Timed from the first ranged request rather than from the call: resolution and
        // the disk check take their own time, and a cancel spent on those proves nothing.
        while watcher.ranges().is_empty() {
            thread::sleep(Duration::from_millis(5));
        }
        thread::sleep(Duration::from_millis(150));
        trigger.cancel();
    });

    let outcome = download::download(&spec(server.url(), dest.clone()), &control, &Silent);

    assert!(
        outcome.is_err(),
        "a cancelled transfer must not report success"
    );
    assert!(!dest.exists(), "no final file until the transfer completes");

    let sidecar = std::fs::read_to_string(dir.join("model.gguf.part.json"))
        .expect("cancelling must leave a sidecar");
    let recorded: serde_json::Value =
        serde_json::from_str(&sidecar).expect("the sidecar must be readable JSON");
    let progress: u64 = recorded["segments"]
        .as_array()
        .expect("segments")
        .iter()
        .map(|segment| segment["completed"].as_u64().expect("completed"))
        .sum();

    // The number is the point, not the key: a sidecar recording zero is a sidecar that
    // records nothing, and one recording the total is a transfer that was not cancelled.
    assert!(
        progress > 0 && progress < body.len() as u64,
        "the sidecar must record partial progress, got {progress} of {}",
        body.len()
    );

    let before = server.ranges().len();

    // The sidecar's whole purpose: a fresh run finishes the job.
    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("resume after cancel");

    let resumed = &server.ranges()[before..];
    assert!(
        !resumed.is_empty() && resumed.iter().all(|(start, _)| *start > 0),
        "every resumed segment must start behind the bytes it already has, saw {resumed:?}"
    );
    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// A server may answer a ranged request with the whole file, and a caching proxy does.
/// Written at a segment's own offset that body overruns the file and overwrites what the
/// other segments fetched — while every counter says the transfer succeeded.
#[test]
fn refuses_a_server_that_ignores_the_range_request() {
    let body = body_of(64_000);
    let server = start_with(
        body.clone(),
        Opts {
            ignores_ranges: true,
            ..Opts::default()
        },
    );
    let dir = scratch("ignored-range");
    let dest = dir.join("model.gguf");

    let error = download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect_err("a server that ignores Range must not be treated as a success");

    let lowered = error.to_lowercase();
    assert!(
        lowered.contains("range") || lowered.contains("200"),
        "the error should name the ignored range, got {error}"
    );
    assert!(!dest.exists(), "nothing may be renamed into place");
    let part = dir.join("model.gguf.part");
    if let Ok(metadata) = std::fs::metadata(&part) {
        assert_eq!(
            metadata.len(),
            body.len() as u64,
            "no segment may write past the end of the file"
        );
    }
}

/// Framing a body by connection close is legal, and it leaves the engine unable to tell
/// a finished transfer from a truncated one — the same trap as a server without ranges.
#[test]
fn refuses_a_server_that_does_not_report_a_size() {
    let body = body_of(64_000);
    let server = start_with(
        body,
        Opts {
            no_size: true,
            ..Opts::default()
        },
    );
    let dir = scratch("nosize");
    let dest = dir.join("model.gguf");

    let error = download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect_err("a transfer with no known size must be refused");

    let lowered = error.to_lowercase();
    assert!(
        lowered.contains("size") || lowered.contains("resume"),
        "the error should say the size is unknown, got {error}"
    );
    assert!(!dest.exists());
    assert!(
        server.ranges().is_empty(),
        "nothing should be transferred, saw {:?}",
        server.ranges()
    );
}

/// A `.part` truncated or partly restored out of band leaves a sidecar describing a file
/// that no longer exists. Believing it renames a file full of holes.
#[test]
fn a_sidecar_that_does_not_cover_the_file_is_replanned() {
    let body = body_of(64_000);
    let server = start(body.clone());
    let dir = scratch("short-sidecar");
    let dest = dir.join("model.gguf");

    std::fs::write(dir.join("model.gguf.part"), vec![0u8; 64_000]).expect("seed part");
    std::fs::write(
        dir.join("model.gguf.part.json"),
        format!(
            r#"{{"sourceUrl":"{}","total":64000,"etag":"{}","segments":[
                {{"start":0,"end":15999,"completed":16000}},
                {{"start":16000,"end":31999,"completed":16000}},
                {{"start":32000,"end":47999,"completed":16000}}]}}"#,
            server.url(),
            digest_of(&body)
        ),
    )
    .expect("seed sidecar");

    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("a sidecar that describes another file must be replanned, not trusted");

    assert!(
        server.ranges().iter().any(|(start, _)| *start == 0),
        "the whole file must be refetched, saw {:?}",
        server.ranges()
    );
    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// The `.part` is preallocated to the full size, so a shorter one has been truncated or
/// replaced out of band. Trusting the sidecar then extends it with zeros and renames a
/// file with a hole in it, while every counter reports success.
#[test]
fn a_part_shorter_than_the_sidecar_claims_is_replanned() {
    let body = body_of(64_000);
    let server = start(body.clone());
    let dir = scratch("short-part");
    let dest = dir.join("model.gguf");

    std::fs::write(dir.join("model.gguf.part"), &body[..8_000]).expect("seed part");
    std::fs::write(
        dir.join("model.gguf.part.json"),
        format!(
            r#"{{"sourceUrl":"{}","total":64000,"etag":"{}","segments":[
                {{"start":0,"end":15999,"completed":16000}},
                {{"start":16000,"end":31999,"completed":0}},
                {{"start":32000,"end":47999,"completed":0}},
                {{"start":48000,"end":63999,"completed":0}}]}}"#,
            server.url(),
            digest_of(&body)
        ),
    )
    .expect("seed sidecar");

    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("download");

    assert!(
        server.ranges().iter().any(|(start, _)| *start == 0),
        "bytes the sidecar calls complete are not on disk and must be refetched, saw {:?}",
        server.ranges()
    );
    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// An inverted range makes a segment's length underflow, which is a panic in debug and a
/// near-u64::MAX length in release.
#[test]
fn a_sidecar_with_an_inverted_range_is_replanned() {
    let body = body_of(64_000);
    let server = start(body.clone());
    let dir = scratch("inverted-sidecar");
    let dest = dir.join("model.gguf");

    std::fs::write(dir.join("model.gguf.part"), vec![0u8; 64_000]).expect("seed part");
    std::fs::write(
        dir.join("model.gguf.part.json"),
        format!(
            r#"{{"sourceUrl":"{}","total":64000,"etag":"{}","segments":[
                {{"start":0,"end":15999,"completed":16000}},
                {{"start":16000,"end":15999,"completed":0}},
                {{"start":32000,"end":47999,"completed":0}},
                {{"start":48000,"end":63999,"completed":0}}]}}"#,
            server.url(),
            digest_of(&body)
        ),
    )
    .expect("seed sidecar");

    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("an impossible segment must be replanned, not measured");

    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// A peer that completes the handshake and then says nothing is not an error the socket
/// will ever report. Resolution runs before there is any transfer to cancel or any range
/// to reissue, so the agent's own read and overall timeouts are the only thing standing
/// between a silent peer and a call that never returns.
#[test]
fn resolution_against_a_silent_server_is_bounded_by_its_timeouts() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("addr").port();
    thread::spawn(move || {
        // Held open rather than dropped: a closed connection is an error, and an error
        // is exactly what this peer never gives.
        let mut accepted = Vec::new();
        for stream in listener.incoming().flatten() {
            accepted.push(stream);
        }
    });

    let dir = scratch("silent");
    let dest = dir.join("model.gguf");
    let url = format!("http://127.0.0.1:{port}/model.gguf");

    let (finished, waiting) = mpsc::channel();
    thread::spawn(move || {
        let _ = finished.send(download::download(
            &spec(url, dest),
            &download::Control::default(),
            &Silent,
        ));
    });

    let outcome = waiting
        .recv_timeout(Duration::from_secs(5))
        .expect("a silent peer must not hold resolution open indefinitely");
    let error = outcome.expect_err("a silent peer cannot be a success");
    assert!(
        error.to_lowercase().contains("timed out"),
        "a timeout is what has to end this, got {error}"
    );
}

/// The throttling this downloader exists to work around, arriving as a status rather
/// than a closed socket.
#[test]
fn retries_a_throttled_segment_instead_of_failing() {
    let body = body_of(64_000);
    let server = start_with(
        body.clone(),
        Opts {
            throttled: 4,
            ..Opts::default()
        },
    );
    let dir = scratch("throttled");
    let dest = dir.join("model.gguf");

    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("a 429 asks the transfer to wait, not to stop");

    assert!(
        server.ranges().len() > 4,
        "each throttled segment should have been retried, saw {:?}",
        server.ranges()
    );
    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// On a multi-hour transfer a segment collects far more than five resets. Counting them
/// for its whole life means the download is guaranteed to give up eventually, however
/// much of it succeeded in between.
#[test]
fn a_segment_that_keeps_making_progress_is_not_abandoned() {
    let body = body_of(400_000);
    let server = start_with(
        body.clone(),
        Opts {
            truncating: 40,
            ..Opts::default()
        },
    );
    let dir = scratch("long-haul");
    let dest = dir.join("model.gguf");

    download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect("a reset that was recovered from is not a step towards giving up");

    assert!(
        server.ranges().len() > 40,
        "every truncated range should have been resumed, saw {}",
        server.ranges().len()
    );
    assert_eq!(std::fs::read(&dest).expect("read result"), body);
}

/// A fatal is the whole transfer's answer, not one segment's. Letting the siblings run to
/// completion first spends most of a 20 GB file's bandwidth on a download that has already
/// failed, and only then reports the failure.
#[test]
fn a_fatal_on_one_segment_stops_the_others() {
    let body = body_of(400_000);
    let server = start_with(
        body,
        Opts {
            missing_first_range: true,
            chunk_delay: Duration::from_millis(25),
            ..Opts::default()
        },
    );
    let dir = scratch("fatal-siblings");
    let dest = dir.join("model.gguf");

    let error = download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect_err("a 404 on one segment must fail the transfer");

    assert!(
        error.contains("404") || error.to_lowercase().contains("not found"),
        "the error should name the status, got {error}"
    );
    // The three siblings hold 300,000 bytes between them. Anything near that is the rest
    // of the file transferred after the transfer was already over.
    assert!(
        server.ranged_bytes() < 100_000,
        "the siblings kept going after the fatal, serving {} of 300000 bytes",
        server.ranged_bytes()
    );
    assert!(!dest.exists());
}

/// A sidecar rejected for describing a `.part` that is too short is rejected in memory
/// only, and `open_part` then restores the `.part` to full length — erasing the very
/// evidence it was rejected on. Until a fresh plan reaches disk, a kill leaves a state the
/// next run accepts, and that run renames a file with a hole in it.
#[test]
fn a_rejected_sidecar_is_replaced_before_the_first_flush() {
    let body = body_of(64_000);
    let stalling = start_with(
        body.clone(),
        Opts {
            stall_always: true,
            ..Opts::default()
        },
    );
    let dir = scratch("rejected-sidecar");
    let dest = dir.join("model.gguf");

    std::fs::write(dir.join("model.gguf.part"), &body[..8_000]).expect("seed part");
    std::fs::write(
        dir.join("model.gguf.part.json"),
        format!(
            r#"{{"sourceUrl":"{}","total":64000,"etag":"{}","segments":[
                {{"start":0,"end":15999,"completed":16000}},
                {{"start":16000,"end":31999,"completed":0}},
                {{"start":32000,"end":47999,"completed":0}},
                {{"start":48000,"end":63999,"completed":0}}]}}"#,
            stalling.url(),
            digest_of(&body)
        ),
    )
    .expect("seed sidecar");

    let control = Arc::new(download::Control::default());
    let running = Arc::clone(&control);
    let url = stalling.url();
    let parked = dest.clone();
    // What is on disk once planning is done is the whole question, so the periodic flush
    // is put out of reach rather than raced.
    let mut unflushed = spec(url, parked);
    unflushed.flush_every = Duration::from_secs(30);
    let worker = thread::spawn(move || download::download(&unflushed, &running, &Silent));

    // Timed from the first ranged request rather than from the call: planning is behind
    // that, and a sleep long enough to cover resolution can also outlast a flush.
    while stalling.ranges().is_empty() {
        thread::sleep(Duration::from_millis(5));
    }
    let killed = scratch("rejected-sidecar-killed");
    std::fs::copy(dir.join("model.gguf.part"), killed.join("model.gguf.part")).expect("copy part");
    std::fs::copy(
        dir.join("model.gguf.part.json"),
        killed.join("model.gguf.part.json"),
    )
    .expect("copy sidecar");
    control.cancel();
    let _ = worker.join();

    // The copy is what a hard kill would have left behind. Finishing from it against a
    // healthy server has to produce the file, not one holed where the stale sidecar
    // claimed bytes that were never fetched.
    let healthy = start(body.clone());
    download::download(
        &spec(healthy.url(), killed.join("model.gguf")),
        &download::Control::default(),
        &Silent,
    )
    .expect("a killed replan must be resumable into the right file");

    assert_eq!(
        std::fs::read(killed.join("model.gguf")).expect("read result"),
        body
    );
}

/// The `.part` is preallocated sparse, so on every run after the first its length is the
/// full size of the file while it holds almost nothing.
#[test]
fn refuses_a_file_the_disk_cannot_hold_when_the_part_already_exists() {
    let body = body_of(64_000);
    let server = start_with(
        body,
        Opts {
            huge: true,
            ..Opts::default()
        },
    );
    let dir = scratch("disk-resume");
    let dest = dir.join("model.gguf");

    let part = std::fs::File::create(dir.join("model.gguf.part")).expect("seed part");
    part.set_len(PETABYTE).expect("preallocate sparse");
    drop(part);

    let error = download::download(
        &spec(server.url(), dest.clone()),
        &download::Control::default(),
        &Silent,
    )
    .expect_err("a file larger than the disk must be refused on resume too");

    let lowered = error.to_lowercase();
    assert!(
        lowered.contains("space") || lowered.contains("disk"),
        "the error should say the disk is the problem, got {error}"
    );
    assert!(
        server.ranges().is_empty(),
        "nothing should be transferred before the check, saw {:?}",
        server.ranges()
    );
}

/// A bar that stops at 99.8% and is then replaced by a finished download is the same
/// defect as one that never moved: the number the screen ends on has to be the file.
#[test]
fn transfer_progress_climbs_monotonically_to_exactly_the_total() {
    let body = body_of(400_000);
    let total = body.len() as u64;
    let server = start_paced(body, Duration::from_millis(25));
    let dir = scratch("progress-total");
    let dest = dir.join("model.gguf");

    let recorder = Recorder::default();
    let mut watched = spec(server.url(), dest.clone());
    watched.progress_every = Duration::from_millis(300);

    download::download(&watched, &download::Control::default(), &recorder).expect("download");

    let all = recorder.all();
    assert_eq!(
        all.first().map(|report| report.phase),
        Some(Phase::Resolving),
        "the transfer is unaccounted for until resolution reports, saw {all:?}"
    );

    let transferring = recorder.phase(Phase::Transferring);
    assert_eq!(transferring[0].completed, 0, "saw {transferring:?}");
    for pair in transferring.windows(2) {
        assert!(
            pair[1].completed >= pair[0].completed,
            "progress went backwards, saw {transferring:?}"
        );
    }
    assert!(
        transferring
            .iter()
            .all(|report| report.total == Some(total)),
        "every report must carry the size, saw {transferring:?}"
    );
    // Endpoints alone would be satisfied by reporting nothing at all in between, which
    // is what a multi-hour transfer looks like when it has hung.
    assert!(
        transferring
            .iter()
            .any(|report| report.completed > 0 && report.completed < total),
        "nothing was reported mid-transfer, saw {transferring:?}"
    );
    assert_eq!(
        transferring.last().expect("a final report").completed,
        total,
        "the last report must be the whole file, saw {transferring:?}"
    );
}

/// A transfer picked up at 90% opens at 90%. Reporting from zero would tell the user the
/// resume they were promised did not happen, and would price the bytes already on disk
/// into the first rate it reported.
#[test]
fn a_resumed_transfer_reports_from_where_it_left_off() {
    let body = body_of(400_000);
    let total = body.len() as u64;
    let already = 360_000;
    let server = start_paced(body.clone(), Duration::from_millis(50));
    let dir = scratch("progress-resume");
    let dest = dir.join("model.gguf");

    let mut partial = vec![0u8; 400_000];
    partial[..already].copy_from_slice(&body[..already]);
    std::fs::write(dir.join("model.gguf.part"), &partial).expect("seed part");
    std::fs::write(
        dir.join("model.gguf.part.json"),
        format!(
            r#"{{"sourceUrl":"{}","total":400000,"etag":"{}","segments":[
                {{"start":0,"end":99999,"completed":100000}},
                {{"start":100000,"end":199999,"completed":100000}},
                {{"start":200000,"end":299999,"completed":100000}},
                {{"start":300000,"end":399999,"completed":60000}}]}}"#,
            server.url(),
            digest_of(&body)
        ),
    )
    .expect("seed sidecar");

    let recorder = Recorder::default();
    let mut watched = spec(server.url(), dest.clone());
    watched.progress_every = Duration::from_millis(50);

    download::download(&watched, &download::Control::default(), &recorder).expect("download");

    let transferring = recorder.phase(Phase::Transferring);
    assert_eq!(
        transferring[0].completed, already as u64,
        "the first report must be the bytes already held, saw {transferring:?}"
    );
    assert!(
        transferring
            .iter()
            .all(|report| report.completed >= already as u64),
        "no report may fall behind the resume point, saw {transferring:?}"
    );
    assert_eq!(
        transferring.last().expect("a final report").completed,
        total
    );

    // 40 KB remain, served one 4 KB chunk every 50 ms, so nothing honest can read much
    // above 80 KB/s. A rate measured from the start of the file rather than between two
    // samples would have to report the 360 KB already on disk as having just arrived.
    for report in &transferring {
        let rate = report.bytes_per_second.unwrap_or(0.0);
        assert!(
            rate < 300_000.0,
            "{rate} B/s is the resumed bytes being counted as new, saw {transferring:?}"
        );
    }
}

/// A rate is a delta, and one sample is not a delta. Reporting zero for the first one
/// would be a lie a UI cannot tell apart from a transfer that has not started moving.
#[test]
fn no_rate_is_reported_until_there_are_two_samples_to_difference() {
    let body = body_of(400_000);
    let server = start_paced(body, Duration::from_millis(25));
    let dir = scratch("progress-rate");
    let dest = dir.join("model.gguf");

    let recorder = Recorder::default();
    let mut watched = spec(server.url(), dest.clone());
    watched.progress_every = Duration::from_millis(50);

    download::download(&watched, &download::Control::default(), &recorder).expect("download");

    let all = recorder.all();
    assert_eq!(
        all[0].bytes_per_second, None,
        "the very first report has nothing behind it, saw {all:?}"
    );

    let transferring = recorder.phase(Phase::Transferring);
    assert_eq!(
        transferring[0].bytes_per_second, None,
        "the first sample of a phase has nothing to difference against, saw {transferring:?}"
    );
    assert!(
        transferring
            .iter()
            .skip(1)
            .any(|report| report.bytes_per_second.is_some_and(|rate| rate > 0.0)),
        "a moving transfer must report a rate once it has been sampled twice, saw \
         {transferring:?}"
    );
}

/// The rate is the pace now, not the pace since the phase began. A line that ran at
/// 40 MB/s for an hour and then dropped to 1 MB/s would keep reading near 40 under an
/// average, and every ETA drawn from it would stay wrong for hours.
#[test]
fn the_rate_tracks_a_transfer_that_slows_down_mid_phase() {
    let body = body_of(1_200_000);
    let total = body.len() as u64;
    // Four segments of 300 KB, each served fast until its last 30 KB, so the transfer as
    // a whole runs at megabytes a second and then at a fraction of that for its final
    // tenth.
    let server = start_with(
        body,
        Opts {
            chunk_delay: Duration::from_millis(2),
            slows_after: Some((270_000, Duration::from_millis(60))),
            ..Opts::default()
        },
    );
    let dir = scratch("progress-slowdown");
    let dest = dir.join("model.gguf");

    let recorder = Recorder::default();
    let mut watched = spec(server.url(), dest.clone());
    watched.progress_every = Duration::from_millis(50);

    download::download(&watched, &download::Control::default(), &recorder).expect("download");

    let transferring = recorder.phase(Phase::Transferring);
    let early = transferring
        .iter()
        .filter(|report| report.completed < total * 8 / 10)
        .filter_map(|report| report.bytes_per_second)
        .fold(0.0f64, f64::max);
    assert!(
        early > 0.0,
        "nothing was measured over the fast stretch, saw {transferring:?}"
    );

    // Slowest of the reports landing after the pace changed. The slowest rather than the
    // last: the closing report can cover a window of a millisecond, which measures
    // scheduling rather than the line.
    let late = transferring
        .iter()
        .filter(|report| report.completed > total - 100_000)
        .filter_map(|report| report.bytes_per_second)
        .fold(f64::INFINITY, f64::min);
    assert!(
        late.is_finite(),
        "nothing was measured over the slow stretch, saw {transferring:?}"
    );

    // The two stretches differ by more than tenfold, so a late figure within five of the
    // early one is an average dragging the collapse back towards what preceded it.
    assert!(
        late * 5.0 < early,
        "{late} B/s at the end against {early} B/s at the start: the rate is an average \
         over the phase rather than the pace now, saw {transferring:?}"
    );
}

/// A 21 GB transfer is about a third of a million reads. Reporting each one buries the
/// UI in events it cannot draw and cannot skip.
#[test]
fn the_reporting_interval_bounds_how_many_reports_a_transfer_emits() {
    let body = body_of(400_000);
    let total = body.len() as u64;

    let count = |interval: Duration, name: &str| -> usize {
        let server = start_paced(body.clone(), Duration::from_millis(25));
        let dir = scratch(name);
        let dest = dir.join("model.gguf");
        let recorder = Recorder::default();
        let mut watched = spec(server.url(), dest.clone());
        watched.progress_every = interval;

        download::download(&watched, &download::Control::default(), &recorder).expect("download");

        let transferring = recorder.phase(Phase::Transferring);
        assert_eq!(
            transferring.last().expect("a final report").completed,
            total,
            "throttling must not cost the endpoint, saw {transferring:?}"
        );
        transferring.len()
    };

    let sparse = count(Duration::from_millis(300), "progress-sparse");
    let dense = count(Duration::from_millis(50), "progress-dense");

    // The body is served in 4 KB chunks, so an unthrottled engine would report scores of
    // times over the same transfer.
    assert!(
        sparse <= 5,
        "a 300 ms interval emitted {sparse} reports over roughly 650 ms of transfer"
    );
    assert!(
        dense >= sparse * 2,
        "the interval did not change the report count: {dense} at 50 ms against {sparse} \
         at 300 ms"
    );
}

/// Hashing 21 GB takes a minute or two. A phase that reports its start and then nothing
/// until it ends is indistinguishable from one that has hung.
#[test]
fn verification_reports_its_way_through_the_file() {
    let body = body_of(400_000);
    let total = body.len() as u64;
    let server = start(body);
    let dir = scratch("progress-verify");
    let dest = dir.join("model.gguf");

    let recorder = Recorder::default();
    let mut watched = spec(server.url(), dest.clone());
    watched.verify = true;
    watched.progress_every = Duration::ZERO;

    download::download(&watched, &download::Control::default(), &recorder).expect("download");

    let verifying = recorder.phase(Phase::Verifying);
    assert!(
        verifying.len() >= 3,
        "verification reported only its endpoints, saw {verifying:?}"
    );
    assert_eq!(verifying[0].completed, 0, "saw {verifying:?}");
    assert_eq!(
        verifying[0].bytes_per_second, None,
        "the first sample of the pass has nothing behind it, saw {verifying:?}"
    );
    for pair in verifying.windows(2) {
        assert!(
            pair[1].completed > pair[0].completed,
            "verification progress went backwards or stood still, saw {verifying:?}"
        );
    }
    assert_eq!(
        verifying.last().expect("a final report").completed,
        total,
        "verification must finish on the whole file, saw {verifying:?}"
    );
    assert!(verifying.iter().all(|report| report.total == Some(total)));
}

/// The floor holds over the hashing pass as much as over the transfer. Verification reads
/// a 21 GB file in 64 KB bites — a third of a million of them — and a phase that reports
/// one per read floods the UI with exactly the events the interval exists to prevent.
#[test]
fn the_reporting_interval_bounds_verification_too() {
    let body = body_of(4 * 1024 * 1024);
    let total = body.len() as u64;
    let server = start(body);
    let dir = scratch("progress-verify-interval");
    let dest = dir.join("model.gguf");

    let recorder = Recorder::default();
    let mut watched = spec(server.url(), dest.clone());
    watched.verify = true;
    // Longer than hashing four megabytes can take, so the only reports the pass is
    // entitled to are the two endpoints, which are marks rather than throttled ticks.
    watched.progress_every = Duration::from_secs(600);
    // The silence tolerance is also the read timeout resolution runs under, and the
    // stand-in can miss a 400ms one while the machine is busy. Nothing here is about
    // timeouts, so it is put out of reach rather than raced.
    watched.stall_after = Duration::from_secs(30);

    download::download(&watched, &download::Control::default(), &recorder).expect("download");

    let verifying = recorder.phase(Phase::Verifying);
    assert_eq!(
        verifying.len(),
        2,
        "the pass emitted {} reports under a ten minute floor, so nothing is bounding it",
        verifying.len()
    );
    assert_eq!(verifying[0].completed, 0, "saw {verifying:?}");
    assert_eq!(verifying[1].completed, total, "saw {verifying:?}");
}
