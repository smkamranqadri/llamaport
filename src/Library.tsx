import { useCallback, useEffect, useMemo, useState } from "react";
import {
  deleteModel,
  getDirInfo,
  listModels,
  runnerStart,
  setFavourite,
} from "./api";
import { formatFileSize, formatMemory, formatRelative } from "./format";
import FirstRun from "./FirstRun";
import { PiIcon, PlayIcon, SearchIcon, StopIcon } from "./icons";
import OwnerAvatar from "./OwnerAvatar";
import PiPanel from "./PiPanel";
import type { DirInfo, ModelEntry, RunnerSnapshot, Telemetry } from "./types";

function Badges({ model }: { model: ModelEntry }) {
  const md = model.metadata;
  return (
    <div className="badges">
      {model.quant && <span className="badge">{model.quant}</span>}
      {md?.expertCount ? <span className="badge badge-moe">MoE</span> : null}
      {md != null && !md.hasChatTemplate && (
        <span
          className="badge badge-warn"
          title="No chat template embedded in the file — llama.cpp will fall back to a generic prompt format unless you supply one with --chat-template"
        >
          no template
        </span>
      )}
    </div>
  );
}

/// The one sentence a row gets, where three separate columns used to sit. A running model
/// says what it is costing and how fast it is answering; a stopped one says how big it is
/// and when it last ran, or that it never has. The verb carries what a bare date could
/// not: "today" for a download reads exactly like "today" for a launch.
function rowStat(
  model: ModelEntry,
  running: boolean,
  telemetry: Telemetry | null,
): string {
  const size = formatFileSize(model.sizeBytes);

  if (running) {
    const parts: string[] = [];
    if (telemetry?.processFootprintBytes != null) {
      parts.push(`${formatMemory(telemetry.processFootprintBytes)} memory`);
    }
    const gen = telemetry?.genTps ?? telemetry?.lastGenTps ?? null;
    if (gen != null && gen > 0) {
      parts.push(`${gen.toFixed(0)} tok/s`);
    }
    if (parts.length > 0) return parts.join(" · ");
    return `${size} · starting up`;
  }

  if (model.lastLaunchedSecs) {
    return `${size} · ran ${formatRelative(model.lastLaunchedSecs)}`;
  }
  if (model.modifiedSecs) {
    return `${size} · added ${formatRelative(model.modifiedSecs)}, never run`;
  }
  return size;
}

/// What a delete is about to take. A shard set is several files and one model, and the
/// count is the part worth showing before the question is answered.
function deleteScope(model: ModelEntry): string {
  const parts = model.shards?.present ?? 1;
  const files = parts === 1 ? "1 file" : `${parts} files`;
  return `${files} · ${formatFileSize(model.sizeBytes)}`;
}

/// A running model offers Use in pi and Stop where every other row offers Run. The star
/// and Delete are the artboard's omissions kept alive: neither exists anywhere else in the
/// app, so rather than lose them they wait for a hover. Run sends no draft, so the backend
/// launches on the same remembered profile the model's own screen opens with.
function RowAction({
  model,
  isRunning,
  ready,
  launchable,
  onFavourite,
  onRun,
  onStop,
  onPi,
  onConfirmDelete,
}: {
  model: ModelEntry;
  isRunning: boolean;
  ready: boolean;
  launchable: boolean;
  onFavourite: () => void;
  onRun: () => void;
  onStop: () => void;
  onPi: () => void;
  onConfirmDelete: () => void;
}) {
  // A favourited model shows its star at rest — a mark you cannot see is not a mark.
  const star = (
    <button
      className={`star${model.favourite ? " is-on" : " button-quiet"}`}
      title={model.favourite ? "Remove from favourites" : "Add to favourites"}
      aria-pressed={model.favourite}
      onClick={onFavourite}
    >
      {model.favourite ? "★" : "☆"}
    </button>
  );

  if (isRunning) {
    return (
      <span className="row-actions">
        {star}
        {ready && (
          <button className="button" title="Point pi at this model" onClick={onPi}>
            <PiIcon />
            Use in pi
          </button>
        )}
        <button
          className="button button-danger row-action"
          title="Stop this model"
          onClick={onStop}
        >
          <StopIcon />
          Stop
        </button>
      </span>
    );
  }

  return (
    <span className="row-actions">
      {star}
      <button
        className="button button-danger button-quiet"
        title="Move this model to the Trash"
        onClick={onConfirmDelete}
      >
        Delete
      </button>
      <button
        className="button row-action"
        title={
          launchable
            ? "Run with its last settings"
            : "Stop the running model first — Llamaport runs one at a time"
        }
        disabled={!launchable}
        onClick={onRun}
      >
        <PlayIcon />
        Run
      </button>
    </span>
  );
}

function ModelRow({
  model,
  runner,
  telemetry,
  onSelect,
  onFavourite,
  onDelete,
  onRun,
  onStop,
  onPi,
}: {
  model: ModelEntry;
  runner: RunnerSnapshot;
  telemetry: Telemetry | null;
  onSelect: (model: ModelEntry) => void;
  onFavourite: (model: ModelEntry) => void;
  onDelete: (model: ModelEntry) => void;
  onRun: (model: ModelEntry) => void;
  onStop: () => void;
  onPi: () => void;
}) {
  const incomplete = model.shards && model.shards.missing.length > 0;
  const isRunning =
    runner.modelId === model.id &&
    (runner.state === "starting" || runner.state === "ready");
  const anyRunning = runner.state === "starting" || runner.state === "ready";
  const launchable =
    !anyRunning && !model.error && !incomplete;
  const [confirming, setConfirming] = useState(false);

  // Asked in the row rather than through `window.confirm`: that returns no usable answer
  // in this webview, so a guard on it refuses every delete instead of asking about one.
  if (confirming) {
    return (
      <li className="model-item is-confirming">
        <span className="model-identity">
          <span className="model-name">
            Move {model.displayName} to the Trash?
          </span>
          <span className="model-file">
            {deleteScope(model)} · you can put it back until you empty the Trash
          </span>
        </span>
        <span className="confirm-actions">
          <button className="button" onClick={() => setConfirming(false)}>
            Cancel
          </button>
          <button
            className="button button-danger"
            onClick={() => {
              setConfirming(false);
              onDelete(model);
            }}
          >
            Move to Trash
          </button>
        </span>
      </li>
    );
  }

  // What is wrong with a file replaces the stat rather than sitting under it: the row is
  // one line, and a broken model has nothing worth saying about its size.
  let stat = rowStat(model, isRunning, telemetry);
  let broken = false;
  if (model.error) {
    stat = model.error;
    broken = true;
  } else if (incomplete) {
    const missing = model.shards!.missing;
    stat = `incomplete — missing part${missing.length > 1 ? "s" : ""} ${missing.join(", ")} of ${model.shards!.total}`;
    broken = true;
  }

  return (
    <li className={`model-item${isRunning ? " is-running" : ""}`}>
      <button
        className={`model-row${model.error ? " is-broken" : ""}`}
        onClick={() => onSelect(model)}
      >
        <span className={`dot ${isRunning ? `state-${runner.state}` : "is-idle"}`} />
        <OwnerAvatar owner={model.owner} small />
        <span className="model-name">{model.displayName}</span>
        <Badges model={model} />
        {isRunning && <span className="model-stat">{stat}</span>}
        <span className="row-spacer" />
        {!isRunning && (
          <span className={`model-stat${broken ? " is-broken-stat" : ""}`}>
            {stat}
          </span>
        )}
      </button>

      <RowAction
        model={model}
        isRunning={isRunning}
        ready={isRunning && runner.state === "ready"}
        launchable={launchable}
        onFavourite={() => onFavourite(model)}
        onRun={() => onRun(model)}
        onStop={onStop}
        onPi={onPi}
        onConfirmDelete={() => setConfirming(true)}
      />
    </li>
  );
}

export default function Library({
  runner,
  telemetry,
  onSelect,
  onStop,
  onRunnerChange,
}: {
  runner: RunnerSnapshot;
  telemetry: Telemetry | null;
  onSelect: (model: ModelEntry) => void;
  onStop: () => void;
  onRunnerChange: (snapshot: RunnerSnapshot) => void;
}) {
  const [models, setModels] = useState<ModelEntry[]>([]);
  const [dir, setDir] = useState<DirInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [failure, setFailure] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [pi, setPi] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setFailure(null);
    try {
      const [entries, info] = await Promise.all([listModels(), getDirInfo()]);
      setModels(entries);
      setDir(info);
    } catch (e) {
      setFailure(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const favourite = useCallback((model: ModelEntry) => {
    setFavourite(model.id, !model.favourite)
      .then(setModels)
      .catch((e) => setFailure(String(e)));
  }, []);

  const remove = useCallback((model: ModelEntry) => {
    setFailure(null);
    deleteModel(model.id)
      .then(setModels)
      .catch((e) => setFailure(String(e)));
  }, []);

  const run = useCallback(
    (model: ModelEntry) => {
      setFailure(null);
      runnerStart(model.id)
        .then(onRunnerChange)
        .catch((e) => setFailure(String(e)));
    },
    [onRunnerChange],
  );

  // The search matches the display name and the file name both: a row no longer shows its
  // file, and that is often the half you remember.
  const shown = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return models;
    return models.filter(
      (model) =>
        model.displayName.toLowerCase().includes(needle) ||
        model.fileName.toLowerCase().includes(needle),
    );
  }, [models, query]);

  const onDisk = models.reduce((sum, model) => sum + model.sizeBytes, 0);
  const runningCount =
    runner.state === "starting" || runner.state === "ready" ? 1 : 0;

  const active =
    runner.state === "starting" || runner.state === "ready"
      ? shown.filter((model) => model.id === runner.modelId)
      : [];
  const rest = shown.filter(
    (model) => !active.some((r) => r.id === model.id),
  );
  const groups: { label: string | null; entries: ModelEntry[] }[] =
    active.length > 0
      ? [
          { label: "Running", entries: active },
          { label: "Stopped", entries: rest },
        ]
      : [{ label: null, entries: shown }];

  // What the folder holds, not where it is: the path is Settings' business and the row it
  // sat above no longer shows a file name either.
  const counts: string[] = [];
  if (runningCount > 0) counts.push(`${runningCount} running`);
  counts.push(`${models.length} model${models.length === 1 ? "" : "s"}`);
  counts.push(`${formatFileSize(onDisk)} on disk`);
  const subtitle = counts.join(" · ");

  return (
    <>
      <header className="screen-header">
        <div>
          <h1>Library</h1>
          <p className="screen-subtitle">
            {models.length === 0 && !loading && !failure
              ? "No models yet"
              : subtitle}
          </p>
        </div>
        {models.length > 0 && (
          <span className="search-field">
            <SearchIcon />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search models"
            />
          </span>
        )}
        <button className="button" onClick={() => void load()} disabled={loading}>
          {loading ? "Scanning…" : "Rescan"}
        </button>
      </header>

      {failure && <p className="notice notice-error">{failure}</p>}

      {!loading && !failure && models.length === 0 && <FirstRun dir={dir} />}

      {models.length > 0 && shown.length === 0 && (
        <p className="empty-detail">Nothing here matches “{query.trim()}”.</p>
      )}

      {pi && <PiPanel onClose={() => setPi(false)} />}

      {shown.length > 0 && groups.map(({ label, entries }) => (
        <section key={label ?? "all"}>
          {label && <h2 className="group-label">{label}</h2>}
          <ul className="model-cards">
            {entries.map((model) => (
              <ModelRow
                key={model.id}
                model={model}
                runner={runner}
                telemetry={telemetry}
                onSelect={onSelect}
                onFavourite={favourite}
                onDelete={remove}
                onRun={run}
                onStop={onStop}
                onPi={() => setPi(true)}
              />
            ))}
          </ul>
        </section>
      ))}
    </>
  );
}
