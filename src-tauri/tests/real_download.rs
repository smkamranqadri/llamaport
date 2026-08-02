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
