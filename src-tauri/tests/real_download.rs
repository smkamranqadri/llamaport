//! A real transfer against Hugging Face, ignored by default: it needs the network and
//! moves ~676 MB.
//!
//! The stand-in server proves the engine's logic; only this proves the assumptions it
//! rests on — that the redirect carries `x-linked-size` and a sha256 `x-linked-etag`,
//! that the CDN honours ranges, and that a killed transfer resumes from what is on disk.
//!
//!   cargo test --manifest-path src-tauri/Cargo.toml --test real_download -- --ignored --nocapture

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use llamaport_lib::download::{
    self, Control, Phase, Progress, ProgressSink, Spec, DEFAULT_FLUSH_EVERY, DEFAULT_PROGRESS_EVERY,
};

const URL: &str = "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q8_0.gguf";
const TOTAL: u64 = 675_710_816;
const DIGEST: &str = "ca59ca7f13d0e15a8cfa77bd17e65d24f6844b554a7b6c12e07a5f89ff76844e";

/// Cancels once the transfer has moved `after` bytes, so the kill lands mid-flight
/// rather than at a wall-clock guess.
struct CancelAfter<'a> {
    control: &'a Control,
    after: u64,
    first_seen: AtomicU64,
    peak: AtomicU64,
}

impl ProgressSink for CancelAfter<'_> {
    fn report(&self, progress: Progress) {
        if progress.phase != Phase::Transferring {
            return;
        }
        let _ = self.first_seen.compare_exchange(
            u64::MAX,
            progress.completed,
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
        self.peak.fetch_max(progress.completed, Ordering::Relaxed);
        if progress.completed >= self.after {
            self.control.cancel();
        }
    }
}

#[derive(Default)]
struct Watch {
    first_transferring: Mutex<Option<u64>>,
    rates: Mutex<Vec<f64>>,
    verified: AtomicU64,
}

impl ProgressSink for Watch {
    fn report(&self, progress: Progress) {
        match progress.phase {
            Phase::Transferring => {
                let mut first = self.first_transferring.lock().expect("lock");
                if first.is_none() {
                    *first = Some(progress.completed);
                }
                if let Some(rate) = progress.bytes_per_second {
                    self.rates.lock().expect("lock").push(rate);
                }
            }
            Phase::Verifying => {
                self.verified
                    .fetch_max(progress.completed, Ordering::Relaxed);
            }
            Phase::Resolving => {}
        }
    }
}

fn spec(dest: std::path::PathBuf) -> Spec {
    Spec {
        url: URL.to_string(),
        dest,
        segments: 4,
        stall_after: Duration::from_secs(30),
        retry_backoff: Duration::from_millis(500),
        verify: true,
        progress_every: DEFAULT_PROGRESS_EVERY,
        flush_every: DEFAULT_FLUSH_EVERY,
    }
}

fn digest_of(path: &std::path::Path) -> String {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = std::fs::File::open(path).expect("open result");
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let read = file.read(&mut buffer).expect("read result");
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    format!("{:x}", hasher.finalize())
}

#[test]
#[ignore = "downloads ~676 MB from Hugging Face"]
fn a_real_transfer_survives_being_killed_and_resumes_to_a_verified_file() {
    let dir = std::env::temp_dir().join(format!("llama-hub-real-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let dest = dir.join("qwen2.5-0.5b-instruct-q8_0.gguf");
    let part = dir.join("qwen2.5-0.5b-instruct-q8_0.gguf.part");

    // Kill it a fifth of the way in — far enough that resume has real work to skip.
    let stop_at = TOTAL / 5;
    let control = Control::default();
    let killer = CancelAfter {
        control: &control,
        after: stop_at,
        first_seen: AtomicU64::new(u64::MAX),
        peak: AtomicU64::new(0),
    };

    let interrupted = download::download(&spec(dest.clone()), &control, &killer);
    let reached = killer.peak.load(Ordering::Relaxed);

    assert!(
        interrupted.is_err(),
        "a cancelled transfer must not report success"
    );
    assert!(!dest.exists(), "no final file until the transfer completes");
    assert!(part.exists(), "the partial must survive the kill");
    assert!(
        reached >= stop_at,
        "expected to reach {stop_at} before cancelling, got {reached}"
    );
    println!("killed after {reached} of {TOTAL} bytes");

    // Resume: a fresh control, and a sink that records where the transfer picks up.
    let watch = Watch::default();
    let resumed = download::download(&spec(dest.clone()), &Control::default(), &watch);
    resumed.expect("the resumed transfer must complete");

    let opened_at = watch.first_transferring.lock().expect("lock").unwrap_or(0);
    assert!(
        opened_at > 0,
        "the resumed transfer reported from zero, so it refetched the file"
    );
    println!("resumed at {opened_at} bytes");

    let size = std::fs::metadata(&dest).expect("result metadata").len();
    assert_eq!(size, TOTAL, "final size");
    assert_eq!(digest_of(&dest), DIGEST, "sha256 of the delivered file");
    assert!(
        !part.exists(),
        "the partial should be gone once the file lands"
    );
    assert_eq!(
        watch.verified.load(Ordering::Relaxed),
        TOTAL,
        "verification should report its way to the end of the file"
    );

    let rates = watch.rates.lock().expect("lock");
    println!(
        "{} rate samples, first {:?}",
        rates.len(),
        rates.first().copied()
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// A manager that has never seen the transfer picks it up from the disk and finishes it.
///
/// The engine's own resume is proved above. What this adds is the part parcel 1 exists
/// for: the app closing, everything it held in memory going with it, and a new one
/// finding the bytes anyway. The first manager is dropped entirely rather than killed —
/// this process cannot exit and keep testing — so what is proved is that nothing but the
/// `.part` and its sidecar is needed to continue, which is the property that matters.
#[test]
#[ignore = "downloads ~676 MB from Hugging Face"]
fn a_partial_left_behind_is_adopted_by_a_new_manager_and_resumed_to_a_verified_file() {
    use llamaport_lib::downloads::{DownloadState, Downloads, Options};
    use std::sync::Arc;

    struct Quiet;
    impl llamaport_lib::runner::EventSink for Quiet {
        fn emit(&self, _: &str, _: serde_json::Value) {}
    }

    let dir = std::env::temp_dir().join(format!("llama-hub-adopt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let dest = dir.join("qwen2.5-0.5b-instruct-q8_0.gguf");
    let part = dir.join("qwen2.5-0.5b-instruct-q8_0.gguf.part");

    let stop_at = TOTAL / 5;
    {
        let first = Downloads::new(Arc::new(Quiet), Arc::new(|| {}));
        let started = first
            .start(URL, &dir, &Options::default())
            .expect("admitted");
        let id = started[0].id.clone();

        let deadline = std::time::Instant::now() + Duration::from_secs(300);
        loop {
            let moved = first
                .snapshot()
                .into_iter()
                .find(|job| job.id == id)
                .map(|job| job.completed)
                .unwrap_or(0);
            if moved >= stop_at {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the transfer never reached {stop_at} bytes"
            );
            std::thread::sleep(Duration::from_millis(100));
        }

        first.pause(&id).expect("paused");
        let deadline = std::time::Instant::now() + Duration::from_secs(60);
        while first.snapshot()[0].state == DownloadState::Active {
            assert!(
                std::time::Instant::now() < deadline,
                "the transfer never stopped"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        println!("stopped at {} bytes", first.snapshot()[0].completed);
    }

    assert!(part.exists(), "the partial must outlive the manager");

    // Nothing is carried over: this one is built from scratch, exactly as the app is on a
    // restart, and everything it knows it reads off the disk.
    let second = Downloads::new(Arc::new(Quiet), Arc::new(|| {}));
    assert!(second.snapshot().is_empty(), "a new manager knows nothing");

    let adopted = second.adopt(&dir);
    assert_eq!(adopted.len(), 1, "the partial on disk was not found");
    assert_eq!(adopted[0].state, DownloadState::Paused);
    assert_eq!(adopted[0].url, URL, "the source came back from the sidecar");
    assert_eq!(adopted[0].total, Some(TOTAL));
    assert!(adopted[0].resumable);
    assert!(
        adopted[0].completed >= stop_at,
        "adopted at {} bytes, which is behind where it stopped",
        adopted[0].completed
    );
    println!("adopted at {} of {TOTAL} bytes", adopted[0].completed);

    let id = adopted[0].id.clone();
    second
        .resume(&id, &dir, &Options::default())
        .expect("resumed");

    let deadline = std::time::Instant::now() + Duration::from_secs(900);
    loop {
        let job = second
            .snapshot()
            .into_iter()
            .find(|job| job.id == id)
            .expect("tracked");
        if job.state != DownloadState::Active {
            assert_eq!(job.state, DownloadState::Complete, "{:?}", job.error);
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the resumed transfer never finished"
        );
        std::thread::sleep(Duration::from_millis(200));
    }

    assert_eq!(digest_of(&dest), DIGEST, "sha256 of the delivered file");
    assert!(!part.exists(), "the partial is gone once the file lands");

    let _ = std::fs::remove_dir_all(&dir);
}

/// Discard against a live transfer, where the writer is a real one holding a real file
/// open. The stand-in engine proves the ordering; this proves the bytes actually go.
#[test]
#[ignore = "fetches part of a 676 MB file from Hugging Face"]
fn discarding_a_live_transfer_takes_the_part_file_with_it() {
    use llamaport_lib::downloads::{Downloads, Options};
    use std::sync::Arc;

    struct Quiet;
    impl llamaport_lib::runner::EventSink for Quiet {
        fn emit(&self, _: &str, _: serde_json::Value) {}
    }

    let dir = std::env::temp_dir().join(format!("llama-hub-discard-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let part = dir.join("qwen2.5-0.5b-instruct-q8_0.gguf.part");
    let sidecar = dir.join("qwen2.5-0.5b-instruct-q8_0.gguf.part.json");

    let downloads = Downloads::new(Arc::new(Quiet), Arc::new(|| {}));
    let started = downloads
        .start(URL, &dir, &Options::default())
        .expect("admitted");
    let id = started[0].id.clone();

    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    loop {
        let moved = downloads
            .snapshot()
            .into_iter()
            .find(|job| job.id == id)
            .map(|job| job.completed)
            .unwrap_or(0);
        if moved > 8 * 1024 * 1024 {
            println!("discarding a transfer that has moved {moved} bytes");
            break;
        }
        assert!(std::time::Instant::now() < deadline, "never got going");
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(part.exists() && sidecar.exists());

    downloads.discard(&id).expect("discarded");

    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    while !downloads.snapshot().is_empty() {
        assert!(
            std::time::Instant::now() < deadline,
            "the discarded row was never dropped"
        );
        std::thread::sleep(Duration::from_millis(50));
    }

    assert!(!part.exists(), "the part file outlived the discard");
    assert!(!sidecar.exists(), "the sidecar outlived the discard");
    assert_eq!(
        std::fs::read_dir(&dir).expect("read dir").count(),
        0,
        "the models directory should be as it was"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
