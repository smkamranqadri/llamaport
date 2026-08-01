import { useCallback, useEffect, useState } from "react";
import { getDirInfo, listModels } from "./api";
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

function ModelRow({
  model,
  runner,
  onSelect,
}: {
  model: ModelEntry;
  runner: RunnerSnapshot;
  onSelect: (model: ModelEntry) => void;
}) {
  const md = model.metadata;
  const incomplete = model.shards && model.shards.missing.length > 0;
  const isRunning =
    runner.modelId === model.id &&
    (runner.state === "starting" || runner.state === "ready");

  return (
    <li>
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
          <p className="empty-title">
            {dir?.exists ? "No GGUF files here" : "Models directory not found"}
          </p>
          <p className="empty-detail">{dir?.path}</p>
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
            />
          ))}
        </ul>
      )}
    </>
  );
}
