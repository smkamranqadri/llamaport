import { useCallback, useEffect, useState } from "react";
import type { FormEvent } from "react";
import {
  downloadCancel,
  downloadClear,
  downloadStart,
  downloadStatus,
  getDirInfo,
  getSettings,
  onDownloadProgress,
  onDownloadState,
  setDownloadOptions,
} from "./api";
import { formatBytes, formatRate } from "./format";
import type {
  DirInfo,
  DownloadJob,
  DownloadOptions,
  DownloadPhase,
} from "./types";

type Recovery = "resume" | "restart" | "none";

const MIB = 1024 ** 2;

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
  if (job.state === "cancelled") return "resume";
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

function badgeFor(job: DownloadJob): { text: string; className: string } {
  if (job.state === "active") {
    if (job.phase == null) return { text: "starting", className: "badge" };
    return { text: PHASE_LABEL[job.phase], className: "badge badge-moe" };
  }
  if (job.state === "complete") return { text: "complete", className: "badge" };
  if (job.state === "cancelled") {
    return { text: "cancelled", className: "badge badge-quiet" };
  }
  return { text: "failed", className: "badge badge-warn" };
}

function detail(job: DownloadJob): string {
  if (job.state === "complete") {
    return `In the models directory · ${formatBytes(job.completed)}`;
  }
  if (job.state === "active") {
    if (job.phase == null) return "Starting…";
    if (job.phase === "resolving") return "Asking Hugging Face for the file…";
    if (job.phase === "verifying") {
      return `Reading the file back to check its digest · ${moved(job)} · ${rateText(job.bytesPerSecond)}`;
    }
    return `${moved(job)} · ${rateText(job.bytesPerSecond)}`;
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
  busy,
  blocked,
  onCancel,
  onRetry,
  onShow,
}: {
  job: DownloadJob;
  busy: boolean;
  blocked: boolean;
  onCancel: (id: string) => void;
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
            <button
              className="button button-danger"
              onClick={() => onCancel(job.id)}
            >
              Cancel
            </button>
          )}
          {mode !== "none" && (
            <button
              className="button"
              disabled={busy || blocked}
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
      <p className="field-hint">{detail(job)}</p>
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

/// One row per file: a resume starts a fresh job for a URL that already has one, and two
/// rows for the same file read as a bug rather than as history.
function newestPerUrl(jobs: DownloadJob[]): DownloadJob[] {
  const byUrl = new Map<string, DownloadJob>();
  jobs.forEach((job) => byUrl.set(job.url, job));
  return [...byUrl.values()].reverse();
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
      onDownloadProgress((progress) =>
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
        ),
      ),
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

  const cancel = useCallback((id: string) => {
    setFailure(null);
    downloadCancel(id)
      .then(setJobs)
      .catch((e) => setFailure(String(e)));
  }, []);

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
  const rows = newestPerUrl(jobs);
  const settled = rows.some((job) => job.state !== "active");

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
        {settled && (
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
          <button
            className="button button-primary"
            type="submit"
            disabled={busy || active != null}
          >
            Download
          </button>
        </form>
        {active && (
          <p className="field-hint">
            One file at a time — {active.fileName} is still downloading.
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

      {rows.length === 0 && (
        <div className="empty">
          <p className="empty-title">Nothing downloaded yet</p>
          <p className="empty-detail">
            Paste the URL of a .gguf file on Hugging Face to fetch it into the
            models directory.
          </p>
        </div>
      )}

      {rows.length > 0 && (
        <ul className="model-list">
          {rows.map((job) => (
            <Row
              key={job.id}
              job={job}
              busy={busy}
              blocked={active != null}
              onCancel={cancel}
              onRetry={(target) => void request(target)}
              onShow={onShowInLibrary}
            />
          ))}
        </ul>
      )}
    </>
  );
}
