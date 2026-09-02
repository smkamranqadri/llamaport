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
  listModels,
  onDownloadProgress,
  onDownloadState,
  setDownloadOptions,
} from "./api";
import { formatDuration, formatFileSize, formatRate, MB } from "./format";
import { CheckIcon, CloseIcon, DownloadIcon, SearchIcon } from "./icons";
import type {
  DirInfo,
  DownloadJob,
  DownloadOptions,
  DownloadPhase,
  ModelEntry,
} from "./types";

type Recovery = "resume" | "restart" | "none";

/// Finished rows shown at once. The history is never trimmed — a page is how a list of
/// every model ever fetched stays readable.
const PAGE = 25;


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

/// The named limits, because a limit is chosen and not measured: nobody types 37 MB/s.
/// A figure already in the config that is not one of these joins the list rather than
/// being rounded away.
const LIMITS_MB = [5, 10, 25, 50, 100];

function isQuantToken(token: string): boolean {
  if (["F16", "BF16", "F32", "F64"].includes(token)) return true;
  const rest = token.replace(/^(IQ|TQ|Q)/, "");
  return rest !== token && /^\d/.test(rest);
}

/// The same rule `catalog.rs` uses on a file that has landed, applied to one that has
/// not: a row in flight has no GGUF to read, and the badge beside its name has to come
/// from somewhere.
function quantOf(fileName: string): string | null {
  const tokens = fileName.replace(/\.gguf$/i, "").split(/[-.]/);
  for (let i = 0; i < tokens.length; i += 1) {
    const upper = tokens[i].toUpperCase();
    if (!isQuantToken(upper)) continue;
    if (i > 0 && tokens[i - 1].toUpperCase() === "UD") return `UD-${upper}`;
    return upper;
  }
  return null;
}

function stem(fileName: string) {
  return fileName.replace(/\.gguf$/i, "");
}

function percent(job: DownloadJob): number | null {
  if (job.total == null || job.total === 0) return null;
  return Math.min(100, Math.floor((job.completed / job.total) * 100));
}

function moved(job: DownloadJob): string {
  if (job.total == null) return formatFileSize(job.completed);
  return `${formatFileSize(job.completed)} of ${formatFileSize(job.total)}`;
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

/// The one line of figures under a transfer. `place` is where this job sits in the
/// queue, counting from one, and zero for a job that is not in it.
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

/// A transfer, as the artboard draws it: the name it will have, what it is, the two
/// actions, a bar, and one line of figures. Everything else this screen used to print
/// about a healthy download is in that line.
function Downloading({
  job,
  name,
  smoothed,
  place,
  busy,
  onPause,
  onResume,
  onDiscard,
}: {
  job: DownloadJob;
  name: string;
  smoothed: Smoothed | undefined;
  place: number;
  busy: boolean;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onDiscard: (id: string) => void;
}) {
  const quant = quantOf(job.fileName);
  const done = percent(job);
  const bar =
    job.state === "active" &&
    job.phase != null &&
    job.phase !== "resolving" &&
    done != null;

  let discardHint = "Stop this transfer and delete the bytes it has fetched";
  if (job.state === "queued") discardHint = "Take this out of the queue";

  return (
    <div className="download-card">
      <div className="download-head">
        <span className="download-name">{name}</span>
        {quant && <span className="badge">{quant}</span>}
        <span className="actions">
          {job.state === "active" && (
            <button className="button button-plain" onClick={() => onPause(job.id)}>
              Pause
            </button>
          )}
          {job.state === "paused" && job.resumable && (
            <button
              className="button button-plain"
              disabled={busy}
              onClick={() => onResume(job.id)}
            >
              Resume
            </button>
          )}
          <button
            className="button button-plain button-danger"
            title={discardHint}
            onClick={() => onDiscard(job.id)}
          >
            <CloseIcon />
            Cancel
          </button>
        </span>
      </div>

      {bar && (
        <div className={`bar download-bar${job.phase === "verifying" ? " is-verifying" : ""}`}>
          <span style={{ width: `${done}%` }} />
        </div>
      )}

      <span className="card-sub">{detail(job, smoothed, place)}</span>
    </div>
  );
}

/// One that did not make it. The URL is printed here and nowhere else on this screen:
/// a failure is the one moment somebody needs to read the address they asked for.
function Failed({
  job,
  name,
  busy,
  onDiscard,
  onRetry,
}: {
  job: DownloadJob;
  name: string;
  busy: boolean;
  onDiscard: (id: string) => void;
  onRetry: (url: string) => void;
}) {
  const mode = recovery(job);

  return (
    <div className="download-card">
      <div className="download-head">
        <span className="dot tone-bad" />
        <span className="download-name">{name}</span>
        <span className="actions">
          {mode !== "none" && (
            <button
              className="button button-plain"
              disabled={busy}
              onClick={() => onRetry(job.url)}
            >
              {RETRY_LABEL[mode]}
            </button>
          )}
          <button
            className="button button-plain button-danger"
            onClick={() => onDiscard(job.id)}
          >
            <CloseIcon />
            Discard
          </button>
        </span>
      </div>
      <span className="download-url" title={job.url}>
        {job.url}
      </span>
      {job.error && <p className="model-error">{job.error}</p>}
      <span className="card-sub">{detail(job, undefined, 0)}</span>
    </div>
  );
}

/// A file that landed. Its name is the Library's own once the catalog has seen it, so
/// the two screens cannot call the same file different things.
function Finished({
  job,
  name,
  inLibrary,
  onShow,
}: {
  job: DownloadJob;
  name: string;
  inLibrary: boolean;
  onShow: (path: string) => void;
}) {
  let stat = `In your Library · ${formatFileSize(job.completed)}`;
  if (!inLibrary) {
    stat = `No longer in the models directory · ${formatFileSize(job.completed)}`;
  }

  return (
    <div className="finished-row">
      <span className="finished-tick">
        <CheckIcon />
      </span>
      <span className="download-name" title={job.fileName}>
        {name}
      </span>
      <span className="finished-stat">{stat}</span>
      {inLibrary && (
        <button className="button button-link" onClick={() => onShow(job.path)}>
          Show
        </button>
      )}
    </div>
  );
}

/// What the menu offers, as bytes per second, with the stored figure folded in if it is
/// not one of the named ones — a limit set by an older build must not vanish because this
/// list does not name it.
function limitChoices(applied: number | null): (number | null)[] {
  const named = LIMITS_MB.map((mb) => mb * MB);
  if (applied != null && !named.includes(applied)) {
    named.push(applied);
    named.sort((a, b) => a - b);
  }
  return [null, ...named];
}

function limitLabel(rate: number | null): string {
  if (rate == null) return "No speed limit";
  return formatRate(rate);
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
  const [models, setModels] = useState<ModelEntry[]>([]);
  const [rates, setRates] = useState<Record<string, Smoothed>>({});
  const [page, setPage] = useState(PAGE);

  useEffect(() => {
    const readDir = () => getDirInfo().then(setDir).catch(() => {});
    downloadStatus()
      .then(setJobs)
      .catch((e) => setFailure(String(e)));
    getSettings()
      .then((settings) => setOptions(settings.downloads))
      .catch((e) => setFailure(String(e)));
    // A finished row is named by the catalog, so the catalog is re-read when one lands.
    const readModels = () => listModels().then(setModels).catch(() => {});
    readDir();
    readModels();

    const unlisten = [
      onDownloadState((next) => {
        setJobs(next);
        readDir();
        readModels();
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

  const applyLimit = (rateLimit: number | null) => {
    if (!options) return;
    setFailure(null);
    setDownloadOptions({ ...options, rateLimit })
      .then((settings) => setOptions(settings.downloads))
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
  const failed = jobs.filter((job) => job.state === "failed").reverse();
  // Newest first, and never trimmed — only ever shown a page at a time.
  const finished = jobs.filter((job) => job.state === "complete").reverse();
  const shown = finished.slice(0, page);

  // A finished file is named by the catalog, which reads the GGUF's own name; one still
  // in flight has no GGUF to read, so its file name stands in until it lands.
  const nameOf = (job: DownloadJob) => {
    const model = models.find((entry) => entry.path === job.path);
    return model?.displayName ?? stem(job.fileName);
  };
  const inLibrary = (job: DownloadJob) =>
    models.some((entry) => entry.path === job.path);

  let status = "Nothing downloading";
  if (unfinished.length === 1) status = "1 downloading";
  if (unfinished.length > 1) status = `${unfinished.length} downloading`;

  return (
    <>
      <header className="screen-header">
        <div>
          <h1>Downloads</h1>
          <p className="screen-subtitle status-line">
            <span>{status}</span>
            {dir?.freeBytes != null && (
              <>
                <span>·</span>
                <span>{formatFileSize(dir.freeBytes)} free</span>
              </>
            )}
            {options && (
              <>
                <span>·</span>
                <select
                  className="limit-select"
                  value={String(options.rateLimit ?? "")}
                  title="How fast a transfer is allowed to go. A change applies to the download running now, not only the next one."
                  onChange={(e) => {
                    const chosen = e.currentTarget.value;
                    applyLimit(chosen === "" ? null : Number(chosen));
                  }}
                >
                  {limitChoices(options.rateLimit).map((rate) => (
                    <option key={String(rate ?? "")} value={String(rate ?? "")}>
                      {limitLabel(rate)}
                    </option>
                  ))}
                </select>
              </>
            )}
          </p>
        </div>

        <form className="get-row" onSubmit={submit}>
          <span className="search-field get-field">
            <SearchIcon />
            <input
              value={url}
              placeholder="Paste a Hugging Face link"
              onChange={(e) => setUrl(e.currentTarget.value)}
            />
          </span>
          <button className="button button-primary" type="submit" disabled={busy}>
            <DownloadIcon />
            Get
          </button>
        </form>
      </header>

      {failure && <p className="notice notice-error">{failure}</p>}

      {jobs.length === 0 && (
        <div className="empty">
          <p className="empty-title">Nothing downloaded yet</p>
          <p className="empty-detail">
            Paste the link to a .gguf file on Hugging Face and press Get. It
            lands in {dir?.path ?? "the models directory"} and appears in your
            Library.
          </p>
        </div>
      )}

      {unfinished.length > 0 && (
        <>
          <h2 className="group-label">Downloading</h2>
          <div className="download-list">
            {unfinished.map((job) => (
              <Downloading
                key={job.id}
                job={job}
                name={nameOf(job)}
                smoothed={rates[job.id]}
                place={queue.indexOf(job) + 1}
                busy={busy}
                onPause={pause}
                onResume={resume}
                onDiscard={discard}
              />
            ))}
          </div>
          <p className="field-hint download-note">
            Downloads survive quitting the app — they come back paused, ready to
            pick up where they left off.
          </p>
          {active && unfinished.length > 1 && (
            <p className="field-hint download-note">
              One file at a time: the rest start when {nameOf(active)} stops.
            </p>
          )}
        </>
      )}

      {failed.length > 0 && (
        <>
          <h2 className="group-label">Did not finish</h2>
          <div className="download-list">
            {failed.map((job) => (
              <Failed
                key={job.id}
                job={job}
                name={nameOf(job)}
                busy={busy}
                onDiscard={discard}
                onRetry={(target) => void request(target)}
              />
            ))}
          </div>
        </>
      )}

      {finished.length > 0 && (
        <>
          <h2 className="group-label">Finished</h2>
          <div className="download-list">
            {shown.map((job) => (
              <Finished
                key={job.id}
                job={job}
                name={nameOf(job)}
                inLibrary={inLibrary(job)}
                onShow={onShowInLibrary}
              />
            ))}
          </div>
          {shown.length < finished.length && (
            <button
              className="button load-more"
              onClick={() => setPage((seen) => seen + PAGE)}
            >
              Load more ({finished.length - shown.length} older)
            </button>
          )}
        </>
      )}

      {/* Below everything it removes, because it removes both lists: the engine counts a
          failure as finished, and a Clear sitting under one group would say otherwise. */}
      {finished.length + failed.length > 0 && (
        <div className="clear-row">
          <button
            className="button button-plain"
            title="Removes every finished and failed row from this list. The files already downloaded are not touched."
            onClick={() =>
              downloadClear()
                .then(setJobs)
                .catch((e) => setFailure(String(e)))
            }
          >
            Clear history
          </button>
        </div>
      )}
    </>
  );
}
