import { useCallback, useEffect, useState } from "react";
import { deleteModel, getDirInfo, listModels, setFavourite } from "./api";
import { formatBytes, formatContext, formatRelative } from "./format";
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

/// What a delete is about to take. A shard set is several files and one model, and the
/// count is the part worth showing before the question is answered.
function deleteScope(model: ModelEntry): string {
  const parts = model.shards?.present ?? 1;
  const files = parts === 1 ? "1 file" : `${parts} files`;
  return `${files} · ${formatBytes(model.sizeBytes)}`;
}

function ModelRow({
  model,
  runner,
  onSelect,
  onFavourite,
  onDelete,
}: {
  model: ModelEntry;
  runner: RunnerSnapshot;
  onSelect: (model: ModelEntry) => void;
  onFavourite: (model: ModelEntry) => void;
  onDelete: (model: ModelEntry) => void;
}) {
  const md = model.metadata;
  const incomplete = model.shards && model.shards.missing.length > 0;
  const isRunning =
    runner.modelId === model.id &&
    (runner.state === "starting" || runner.state === "ready");
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
    <li className="model-item">
      <button
        className={`star${model.favourite ? " is-on" : ""}`}
        title={model.favourite ? "Remove from favourites" : "Add to favourites"}
        aria-pressed={model.favourite}
        onClick={() => onFavourite(model)}
      >
        {model.favourite ? "★" : "☆"}
      </button>

      <button
        className={`model-row${model.error ? " is-broken" : ""}${isRunning ? " is-running" : ""}`}
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

        <span className="model-stat">{formatBytes(model.sizeBytes)}</span>
        <span className="model-stat">
          {md?.contextLength ? formatContext(md.contextLength) : "—"}
        </span>
        <span className="model-stat model-muted">
          {model.modifiedSecs ? formatRelative(model.modifiedSecs) : ""}
        </span>
      </button>

      <button
        className="button button-danger button-quiet"
        title={
          isRunning
            ? "Stop this model before deleting it"
            : "Move this model to the Trash"
        }
        disabled={isRunning}
        onClick={() => setConfirming(true)}
      >
        Delete
      </button>
    </li>
  );
}

export default function Library({
  runner,
  onSelect,
}: {
  runner: RunnerSnapshot;
  onSelect: (model: ModelEntry) => void;
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
            {dir?.freeBytes != null && ` · ${formatBytes(dir.freeBytes)} free`}
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

      {models.length > 0 && (
        <ul className="model-list">
          {models.map((model) => (
            <ModelRow
              key={model.id}
              model={model}
              runner={runner}
              onSelect={onSelect}
              onFavourite={favourite}
              onDelete={remove}
            />
          ))}
        </ul>
      )}
    </>
  );
}
