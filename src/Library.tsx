import { useCallback, useEffect, useState } from "react";
import {
  deleteModel,
  getDirInfo,
  listModels,
  runnerStart,
  setFavourite,
} from "./api";
import { formatContext, formatFileSize, formatRelative } from "./format";
import { PlayIcon, StopIcon } from "./icons";
import type { DirInfo, ModelEntry, RunnerSnapshot } from "./types";

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

/// One cell for two facts: when the model last ran, or when its file arrived if it never
/// has. Told apart by weight rather than by a label, because the list has no header row
/// and "today" for a download reads exactly like "today" for a launch.
function Recency({ model }: { model: ModelEntry }) {
  if (model.lastLaunchedSecs) {
    return (
      <span className="model-stat" title="Last launched">
        {formatRelative(model.lastLaunchedSecs)}
      </span>
    );
  }
  if (model.modifiedSecs) {
    return (
      <span
        className="model-stat model-muted"
        title="Added to the models directory — never launched"
      >
        {formatRelative(model.modifiedSecs)}
      </span>
    );
  }
  return <span className="model-stat model-muted">—</span>;
}

/// What a delete is about to take. A shard set is several files and one model, and the
/// count is the part worth showing before the question is answered.
function deleteScope(model: ModelEntry): string {
  const parts = model.shards?.present ?? 1;
  const files = parts === 1 ? "1 file" : `${parts} files`;
  return `${files} · ${formatFileSize(model.sizeBytes)}`;
}

/// A running model offers Stop where every other row offers Run, with Delete kept but
/// quiet: deleting is rare, and a running model refuses it anyway. Run sends no draft,
/// so the backend launches on the same remembered profile the model's own screen opens
/// with.
function RowAction({
  isRunning,
  launchable,
  onRun,
  onStop,
  onConfirmDelete,
}: {
  isRunning: boolean;
  launchable: boolean;
  onRun: () => void;
  onStop: () => void;
  onConfirmDelete: () => void;
}) {
  if (isRunning) {
    return (
      <span className="row-actions">
        <button
          className="button row-action"
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
  onSelect,
  onFavourite,
  onDelete,
  onRun,
  onStop,
}: {
  model: ModelEntry;
  runner: RunnerSnapshot;
  onSelect: (model: ModelEntry) => void;
  onFavourite: (model: ModelEntry) => void;
  onDelete: (model: ModelEntry) => void;
  onRun: (model: ModelEntry) => void;
  onStop: () => void;
}) {
  const md = model.metadata;
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

  return (
    <li className={`model-item${isRunning ? " is-running" : ""}`}>
      <button
        className={`star${model.favourite ? " is-on" : ""}`}
        title={model.favourite ? "Remove from favourites" : "Add to favourites"}
        aria-pressed={model.favourite}
        onClick={() => onFavourite(model)}
      >
        {model.favourite ? "★" : "☆"}
      </button>

      <button
        className={`model-row${model.error ? " is-broken" : ""}`}
        onClick={() => onSelect(model)}
      >
        <span className="model-identity">
          <span className="model-name">
            {isRunning && <span className={`dot state-${runner.state}`} />}
            {model.displayName}
          </span>
          <span className="model-file" title={model.fileName}>
            {model.fileName}
          </span>
          {model.error && <span className="model-error">{model.error}</span>}
          {incomplete && (
            <span className="model-error">
              incomplete shard set — missing part
              {model.shards!.missing.length > 1 ? "s " : " "}
              {model.shards!.missing.join(", ")} of {model.shards!.total}
            </span>
          )}
        </span>

        <Badges model={model} />

        <span className="model-stat">{formatFileSize(model.sizeBytes)}</span>
        <span className="model-stat">
          {md?.contextLength ? formatContext(md.contextLength) : "—"}
        </span>
        <Recency model={model} />
      </button>

      <RowAction
        isRunning={isRunning}
        launchable={launchable}
        onRun={() => onRun(model)}
        onStop={onStop}
        onConfirmDelete={() => setConfirming(true)}
      />
    </li>
  );
}

export default function Library({
  runner,
  onSelect,
  onStop,
  onRunnerChange,
}: {
  runner: RunnerSnapshot;
  onSelect: (model: ModelEntry) => void;
  onStop: () => void;
  onRunnerChange: (snapshot: RunnerSnapshot) => void;
}) {
  const [models, setModels] = useState<ModelEntry[]>([]);
  const [dir, setDir] = useState<DirInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [failure, setFailure] = useState<string | null>(null);

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

  const active =
    runner.state === "starting" || runner.state === "ready"
      ? models.filter((model) => model.id === runner.modelId)
      : [];
  const rest = models.filter(
    (model) => !active.some((r) => r.id === model.id),
  );
  const groups: { label: string | null; entries: ModelEntry[] }[] =
    active.length > 0
      ? [
          { label: "Running", entries: active },
          { label: "Stopped", entries: rest },
        ]
      : [{ label: null, entries: models }];

  const empty = {
    title: "Models directory not found",
    hint: "Create this folder, or point Settings at the one you keep models in.",
  };
  if (dir?.exists) {
    empty.title = "No GGUF files here";
    empty.hint =
      "Fetch one from Downloads, or point Settings at the folder you keep models in.";
  }

  return (
    <>
      <header className="screen-header">
        <div>
          <h1>Library</h1>
          <p className="screen-subtitle">
            {dir?.path ?? "…"}
            {dir?.freeBytes != null && ` · ${formatFileSize(dir.freeBytes)} free`}
            {models.length > 0 &&
              ` · ${models.length} model${models.length === 1 ? "" : "s"}`}
          </p>
        </div>
        <button className="button" onClick={() => void load()} disabled={loading}>
          {loading ? "Scanning…" : "Rescan"}
        </button>
      </header>

      {failure && <p className="notice notice-error">{failure}</p>}

      {!loading && !failure && models.length === 0 && (
        <div className="empty">
          <p className="empty-title">{empty.title}</p>
          <p className="empty-detail">{dir?.path}</p>
          <p className="empty-detail">{empty.hint}</p>
        </div>
      )}

      {models.length > 0 && groups.map(({ label, entries }) => (
        <section key={label ?? "all"}>
          {label && <h2 className="group-label">{label}</h2>}
          <ul className="model-cards">
            {entries.map((model) => (
              <ModelRow
                key={model.id}
                model={model}
                runner={runner}
                onSelect={onSelect}
                onFavourite={favourite}
                onDelete={remove}
                onRun={run}
                onStop={onStop}
              />
            ))}
          </ul>
        </section>
      ))}
    </>
  );
}
