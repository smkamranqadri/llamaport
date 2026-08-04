import { useCallback, useEffect, useMemo, useState } from "react";
import type { FormEvent } from "react";
import {
  downloadClear,
  downloadDiscard,
  downloadPause,
  downloadResume,
  downloadStart,
  downloadStatus,
  getDirInfo,
  getSettings,
  onDownloadProgress,
  onDownloadState,
  setDownloadOptions,
} from "./api";
import { formatBytes, formatDuration, formatRate } from "./format";
import type {
  DirInfo,
  DownloadJob,
  DownloadOptions,
  DownloadPhase,
} from "./types";

type Recovery = "resume" | "restart" | "none";

/// Finished rows shown at once. The history is never trimmed — a page is how a list of
/// every model ever fetched stays readable.
const PAGE = 25;

const MIB = 1024 ** 2;

/// How much of the smoothed rate survives each new sample. The engine reports twice a
/// second and the raw figure swings hard on an unlimited transfer; dividing the remaining
/// bytes by it would produce an estimate that jumps between two minutes and twenty.
const SMOOTHING = 0.7;

/// Samples needed before an estimate is shown at all. The first second of a transfer knows
/// nothing about how fast it will be.
const SETTLED = 3;

/// A rate the estimate is worth computing from: smoothed, and belonging to the phase that
/// is being estimated. Verification re-reads the file at a different speed, and carrying
/// the transfer rate into it would describe the wrong thing.
interface Smoothed {
  phase: DownloadPhase;
  rate: number;
  samples: number;
}

/// The engine charges a buffer at a time, so anything slower parks a segment for longer
/// than a second and is raised to this on the way in.
const FLOOR = 64 * 1024;

const PHASE_LABEL: Record<DownloadPhase, string> = {
  resolving: "resolving",
  transferring: "transferring",
  verifying: "verifying",
};

const RETRY_LABEL: Record<Recovery, string> = {
  resume: "Resume",
  restart: "Start over",
  none: "",
};

/// The engine refuses these before a byte moves, and refuses them again on every retry.
const UNRESUMABLE = [
  "does not support range requests",
  "did not report the file size",
];

/// The failures arrive as prose, and prose is the only thing there is to read them from.
function recovery(job: DownloadJob): Recovery {
  // A paused transfer is resumed by id rather than by URL, so it is not a recovery at
  // all — except when its bytes are gone, which is the one case it cannot be.
  if (job.state === "paused") return "none";
  if (job.state !== "failed") return "none";

  const cause = job.error ?? "";
  if (UNRESUMABLE.some((marker) => cause.includes(marker))) return "none";
  if (cause.includes("digest mismatch")) return "restart";
  return "resume";
}

function percent(job: DownloadJob): number | null {
  if (job.total == null || job.total === 0) return null;
  return Math.min(100, Math.floor((job.completed / job.total) * 100));
}

function moved(job: DownloadJob): string {
  if (job.total == null) return formatBytes(job.completed);
  return `${formatBytes(job.completed)} of ${formatBytes(job.total)}`;
}

function rateText(bytesPerSecond: number | null): string {
  if (bytesPerSecond == null) return "measuring rate…";
  return formatRate(bytesPerSecond);
}

/// An estimate, or nothing. Nothing is the honest answer more often than it looks: before
/// the rate has settled, on a file whose size upstream never declared, and on a phase with
/// no bytes left to move.
function remainingText(job: DownloadJob, smoothed: Smoothed | undefined): string {
  if (job.total == null) return "";
  if (smoothed == null || smoothed.phase !== job.phase) return "";
  if (smoothed.samples < SETTLED || smoothed.rate <= 0) return "";

  const left = job.total - job.completed;
  if (left <= 0) return "";
  return ` · about ${formatDuration(left / smoothed.rate)} left`;
}

function smooth(
  previous: Smoothed | undefined,
  phase: DownloadPhase,
  rate: number,
): Smoothed {
  if (previous == null || previous.phase !== phase) {
    return { phase, rate, samples: 1 };
  }
  return {
    phase,
    rate: previous.rate * SMOOTHING + rate * (1 - SMOOTHING),
    samples: previous.samples + 1,
  };
}

function badgeFor(job: DownloadJob): { text: string; className: string } {
  if (job.state === "active") {
    if (job.phase == null) return { text: "starting", className: "badge" };
    return { text: PHASE_LABEL[job.phase], className: "badge badge-moe" };
  }
  if (job.state === "complete") return { text: "complete", className: "badge" };
  if (job.state === "queued") {
    return { text: "queued", className: "badge badge-quiet" };
  }
  if (job.state === "paused") {
    return { text: "paused", className: "badge badge-quiet" };
  }
  return { text: "failed", className: "badge badge-warn" };
}

/// `place` is where this job sits in the queue, counting from one, and zero for a job that
/// is not in it.
function detail(
  job: DownloadJob,
  smoothed: Smoothed | undefined,
  place: number,
): string {
  if (job.state === "queued") {
    if (place <= 1) {
      return "Next in line — it starts when the transfer above it stops.";
    }
    const ahead = place - 1;
    return `Waiting its turn — ${ahead} download${ahead === 1 ? "" : "s"} ahead of it.`;
  }
  if (job.state === "complete") {
    return `In the models directory · ${formatBytes(job.completed)}`;
  }
  if (job.state === "active") {
    if (job.phase == null) return "Starting…";
    if (job.phase === "resolving") return "Asking Hugging Face for the file…";

    const left = remainingText(job, smoothed);
    if (job.phase === "verifying") {
      return `Reading the file back to check its digest · ${moved(job)} · ${rateText(job.bytesPerSecond)}${left}`;
    }
    return `${moved(job)} · ${rateText(job.bytesPerSecond)}${left}`;
  }

  if (job.state === "paused") {
    if (!job.resumable) {
      return "The partial file beside this one is gone, so there is nothing left to continue — discard it to clear the sidecar.";
    }
    // `completed` counts the phase, not the transfer: one stopped mid-verify has every
    // byte on disk already, and reporting the digest's read position as progress lies.
    if (job.phase === "verifying") {
      return "Paused while the digest was being checked — the bytes are all here, so resuming re-checks them.";
    }
    if (job.total == null) {
      return "Paused before any bytes moved — resuming starts from the beginning.";
    }
    return `Paused at ${moved(job)} — resuming picks up from there.`;
  }

  const mode = recovery(job);
  if (mode === "resume") {
    // `completed` counts the phase, not the transfer: one stopped mid-verify has every
    // byte on disk already, and reporting the digest's read position as progress lies.
    if (job.phase === "verifying") {
      return "Stopped while the digest was being checked — the bytes are all here, so resuming re-checks them.";
    }
    if (job.total == null) {
      return "Stopped before any bytes moved — resuming starts from the beginning.";
    }
    return `Stopped at ${moved(job)} — resuming picks up from there.`;
  }
  if (mode === "restart") {
    return "The partial file was discarded — this one starts from the beginning.";
  }
  return "Nothing to resume — another attempt would fail the same way.";
}

function Row({
  job,
  smoothed,
  place,
  busy,
  onPause,
  onResume,
  onDiscard,
  onRetry,
  onShow,
}: {
  job: DownloadJob;
  smoothed: Smoothed | undefined;
  place: number;
  busy: boolean;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onDiscard: (id: string) => void;
  onRetry: (url: string) => void;
  onShow: (path: string) => void;
}) {
  const badge = badgeFor(job);
  const mode = recovery(job);
  const done = percent(job);
  const bar =
    job.state === "active" &&
    job.phase != null &&
    job.phase !== "resolving" &&
    done != null;

  let discardHint = "Stop this transfer and delete the bytes it has fetched";
  if (job.state === "queued") discardHint = "Take this out of the queue";

  return (
    <li className="download-row">
      <div className="download-head">
        <span className="model-identity">
          <span className="model-name">{job.fileName}</span>
          <span className="model-file" title={job.url}>
            {job.url}
          </span>
        </span>

        <span className="badges">
          <span className={badge.className}>{badge.text}</span>
        </span>

        <span className="actions">
          {job.state === "active" && (
            <button className="button" onClick={() => onPause(job.id)}>
              Pause
            </button>
          )}
          {job.state === "paused" && job.resumable && (
            <button
              className="button button-primary"
              disabled={busy}
              onClick={() => onResume(job.id)}
            >
              Resume
            </button>
          )}
          {job.state !== "complete" && (
            <button
              className="button button-danger"
              title={discardHint}
              onClick={() => onDiscard(job.id)}
            >
              Discard
            </button>
          )}
          {mode !== "none" && (
            <button
              className="button"
              disabled={busy}
              onClick={() => onRetry(job.url)}
            >
              {RETRY_LABEL[mode]}
            </button>
          )}
          {job.state === "complete" && (
            <button className="button" onClick={() => onShow(job.path)}>
              Show in Library
            </button>
          )}
        </span>
      </div>

      {bar && (
        <div className="telemetry-row download-progress">
          <div
            className={`kv-bar${job.phase === "verifying" ? " is-verifying" : ""}`}
          >
            <span style={{ width: `${done}%` }} />
          </div>
          <span className="telemetry-value">{done}%</span>
        </div>
      )}

      {job.error && <p className="model-error">{job.error}</p>}
      <p className="field-hint">{detail(job, smoothed, place)}</p>
    </li>
  );
}

/// The field holds MB/s because that is how a limit is thought about; the engine holds
/// bytes per second. `formatRate` counts a megabyte as 1024², so this has to as well, or a
/// limit typed as 10 reads back as 9.5.
function toField(bytesPerSecond: number | null): string {
  if (bytesPerSecond == null) return "";
  return String(Math.round((bytesPerSecond / MIB) * 100) / 100);
}

/// `null` for no limit, `undefined` for something that is not a limit at all.
function toRate(typed: string): number | null | undefined {
  const trimmed = typed.trim();
  if (trimmed === "") return null;

  const megabytes = Number(trimmed);
  if (!Number.isFinite(megabytes) || megabytes < 0) return undefined;
  if (megabytes === 0) return null;
  return Math.round(megabytes * MIB);
}

function limitHint(typed: string, applied: number | null): string {
  const asked = toRate(typed);
  if (asked !== undefined && asked !== null && asked < FLOOR) {
    return `Below ${formatRate(FLOOR)}, which is the slowest the engine transfers at — it will use that instead.`;
  }
  if (applied == null) {
    return "No limit. A change applies to the download running now, not only the next one.";
  }
  return `Limited to ${formatRate(applied)}. A change applies to the download running now, not only the next one.`;
}

export default function Downloads({
  onShowInLibrary,
}: {
  onShowInLibrary: (path: string) => void;
}) {
  const [jobs, setJobs] = useState<DownloadJob[]>([]);
  const [dir, setDir] = useState<DirInfo | null>(null);
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [options, setOptions] = useState<DownloadOptions | null>(null);
  const [limit, setLimit] = useState("");
  const [rates, setRates] = useState<Record<string, Smoothed>>({});
  const [page, setPage] = useState(PAGE);

  useEffect(() => {
    const readDir = () => getDirInfo().then(setDir).catch(() => {});
    downloadStatus()
      .then(setJobs)
      .catch((e) => setFailure(String(e)));
    getSettings()
      .then((settings) => {
        setOptions(settings.downloads);
        setLimit(toField(settings.downloads.rateLimit));
      })
      .catch((e) => setFailure(String(e)));
    readDir();

    const unlisten = [
      onDownloadState((next) => {
        setJobs(next);
        readDir();
      }),
      onDownloadProgress((progress) => {
        setJobs((prev) =>
          prev.map((job) => {
            if (job.id !== progress.id) return job;
            return {
              ...job,
              phase: progress.phase,
              completed: progress.completed,
              total: progress.total ?? job.total,
              bytesPerSecond: progress.bytesPerSecond,
            };
          }),
        );

        const rate = progress.bytesPerSecond;
        if (rate == null) return;
        setRates((prev) => ({
          ...prev,
          [progress.id]: smooth(prev[progress.id], progress.phase, rate),
        }));
      }),
    ];

    return () => {
      unlisten.forEach((p) => void p.then((off) => off()));
    };
  }, []);

  const request = useCallback(async (target: string) => {
    setBusy(true);
    setFailure(null);
    try {
      setJobs(await downloadStart(target));
      return true;
    } catch (e) {
      setFailure(String(e));
      return false;
    } finally {
      setBusy(false);
    }
  }, []);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    const target = url.trim();
    if (target === "") return;
    void request(target).then((started) => {
      if (started) setUrl("");
    });
  };

  const act = useCallback(
    (run: (id: string) => Promise<DownloadJob[]>) => (id: string) => {
      setFailure(null);
      run(id)
        .then(setJobs)
        .catch((e) => setFailure(String(e)));
    },
    [],
  );

  const pause = useMemo(() => act(downloadPause), [act]);
  const resume = useMemo(() => act(downloadResume), [act]);
  const discard = useMemo(() => act(downloadDiscard), [act]);

  const applyLimit = (event: FormEvent) => {
    event.preventDefault();
    if (!options) return;

    const rateLimit = toRate(limit);
    if (rateLimit === undefined) {
      setFailure("a speed limit is a number of MB/s, or empty for no limit");
      return;
    }

    setFailure(null);
    setDownloadOptions({ ...options, rateLimit })
      .then((settings) => {
        setOptions(settings.downloads);
        setLimit(toField(settings.downloads.rateLimit));
      })
      .catch((e) => setFailure(String(e)));
  };

  const active = jobs.find((job) => job.state === "active");
  const unfinished = jobs.filter(
    (job) =>
      job.state === "active" ||
      job.state === "queued" ||
      job.state === "paused",
  );
  // The list is the queue's order, so a job's place in it is where it appears among the
  // rows waiting. `indexOf` answers -1 for everything else, which reads as no place.
  const queue = unfinished.filter((job) => job.state === "queued");
  // Newest first, and never trimmed — only ever shown a page at a time.
  const history = jobs.filter((job) => !unfinished.includes(job)).reverse();
  const shown = history.slice(0, page);

  return (
    <>
      <header className="screen-header">
        <div>
          <h1>Downloads</h1>
          <p className="screen-subtitle">
            {dir?.path ?? "…"}
            {dir?.freeBytes != null && ` · ${formatBytes(dir.freeBytes)} free`}
          </p>
        </div>
        {history.length > 0 && (
          <button
            className="button"
            onClick={() =>
              downloadClear()
                .then(setJobs)
                .catch((e) => setFailure(String(e)))
            }
          >
            Clear finished
          </button>
        )}
      </header>

      <section className="panel">
        <h2>Fetch a model</h2>
        <form className="row-input" onSubmit={submit}>
          <input
            value={url}
            placeholder="https://huggingface.co/{repo}/resolve/main/{file}.gguf"
            onChange={(e) => setUrl(e.currentTarget.value)}
          />
          <button className="button button-primary" type="submit" disabled={busy}>
            Download
          </button>
        </form>
        {active && (
          <p className="field-hint">
            One file at a time — {active.fileName} is downloading, and anything
            you add now waits its turn.
          </p>
        )}
      </section>

      {options && (
        <section className="panel">
          <h2>Speed limit</h2>
          <form className="row-input" onSubmit={applyLimit}>
            <input
              value={limit}
              placeholder="MB/s — leave empty for no limit"
              onChange={(e) => setLimit(e.currentTarget.value)}
            />
            <button className="button" type="submit">
              Apply
            </button>
          </form>
          <p className="field-hint">{limitHint(limit, options.rateLimit)}</p>
        </section>
      )}

      {failure && <p className="notice notice-error">{failure}</p>}

      {jobs.length === 0 && (
        <div className="empty">
          <p className="empty-title">Nothing downloaded yet</p>
          <p className="empty-detail">
            Paste the URL of a .gguf file on Hugging Face to fetch it into the
            models directory.
          </p>
        </div>
      )}

      {unfinished.length > 0 && (
        <ul className="model-list">
          {unfinished.map((job) => (
            <Row
              key={job.id}
              job={job}
              smoothed={rates[job.id]}
              place={queue.indexOf(job) + 1}
              busy={busy}
              onPause={pause}
              onResume={resume}
              onDiscard={discard}
              onRetry={(target) => void request(target)}
              onShow={onShowInLibrary}
            />
          ))}
        </ul>
      )}

      {history.length > 0 && (
        <>
          <h2 className="panel-head">
            History · {history.length} download
            {history.length === 1 ? "" : "s"}
          </h2>
          <ul className="model-list">
            {shown.map((job) => (
              <Row
                key={job.id}
                job={job}
                smoothed={rates[job.id]}
                place={0}
                busy={busy}
                onPause={pause}
                onResume={resume}
                onDiscard={discard}
                onRetry={(target) => void request(target)}
                onShow={onShowInLibrary}
              />
            ))}
          </ul>
          {shown.length < history.length && (
            <button
              className="button"
              onClick={() => setPage((seen) => seen + PAGE)}
            >
              Load more ({history.length - shown.length} older)
            </button>
          )}
        </>
      )}
    </>
  );
}
