import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getLaunchPlan, runnerStart, runnerStop, saveProfile } from "./api";
import { formatBytes, formatContext } from "./format";
import ProfileForm, { diffProfile } from "./ProfileForm";
import type {
  LaunchPlan,
  ModelEntry,
  Profile,
  RunnerSnapshot,
  Telemetry,
} from "./types";

const SPARK_POINTS = 60;

function Facts({ model }: { model: ModelEntry }) {
  const md = model.metadata;
  const facts: [string, string][] = [
    ["File", model.fileName],
    ["Size", formatBytes(model.sizeBytes)],
    ["Architecture", md?.architecture ?? "unknown"],
    ["Quantisation", model.quant ?? "unknown"],
    ["Parameters", md?.sizeLabel ?? "—"],
    ["Max context", md?.contextLength ? formatContext(md.contextLength) : "—"],
    ["Layers", md?.blockCount?.toString() ?? "—"],
    ["KV heads", md?.headCountKv?.toString() ?? "—"],
    ["Chat template", md?.hasChatTemplate ? "embedded" : "none"],
    ["Experts", md?.expertCount ? `${md.expertCount} (MoE)` : "dense"],
  ];

  return (
    <dl className="facts">
      {facts.map(([label, value]) => (
        <div key={label} className="fact">
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

function MemoryBar({ plan }: { plan: LaunchPlan }) {
  if (!plan.estimate) {
    return (
      <p className="empty-detail">
        Not enough header metadata to estimate memory for this model.
      </p>
    );
  }

  const { weightsBytes, kvBytes, overheadBytes, totalBytes, calibrated } =
    plan.estimate;
  const scale = Math.max(plan.totalMemory, totalBytes);
  const width = (n: number) => `${(n / scale) * 100}%`;
  const tight = totalBytes > plan.totalMemory * 0.85;

  return (
    <div className="memory">
      <div className="memory-bar">
        <span className="seg seg-weights" style={{ width: width(weightsBytes) }} />
        <span className="seg seg-kv" style={{ width: width(kvBytes) }} />
        <span className="seg seg-overhead" style={{ width: width(overheadBytes) }} />
        <span
          className="memory-limit"
          style={{ left: `${(plan.totalMemory / scale) * 100}%` }}
        />
      </div>
      <p className={`memory-summary${tight ? " is-tight" : ""}`}>
        <strong>{formatBytes(totalBytes)}</strong> of{" "}
        {formatBytes(plan.totalMemory)} installed — weights{" "}
        {formatBytes(weightsBytes)}, KV cache {formatBytes(kvBytes)}, overhead{" "}
        {formatBytes(overheadBytes)}
        {!calibrated && " (uncalibrated)"}
      </p>
    </div>
  );
}

function Sparkline({ values }: { values: number[] }) {
  if (values.length < 2) return null;
  const max = Math.max(...values, 1);
  const points = values
    .map((v, i) => `${(i / (SPARK_POINTS - 1)) * 100},${20 - (v / max) * 20}`)
    .join(" ");

  return (
    <svg className="spark" viewBox="0 0 100 20" preserveAspectRatio="none">
      <polyline points={points} />
    </svg>
  );
}

// Falls back to the server's last-request figure because a bare delta reads 0 the
// instant generation stops, which looks like the number is broken.
function rate(
  live: number | null | undefined,
  last: number | null | undefined,
  digits: number,
) {
  if (live != null && live > 0) return `${live.toFixed(digits)} tok/s`;
  if (last != null && last > 0) return `${last.toFixed(digits)} tok/s last`;
  return "—";
}

function TelemetryPanel({
  runner,
  telemetry,
  history,
}: {
  runner: RunnerSnapshot;
  telemetry: Telemetry | null;
  history: number[];
}) {
  const kv = telemetry?.kvCacheUsage;
  const deferred = telemetry?.requestsDeferred ?? 0;

  return (
    <div className="telemetry">
      <div className="telemetry-row">
        <span className="telemetry-label">KV cache</span>
        <div className="kv-bar">
          <span style={{ width: `${Math.min(100, (kv ?? 0) * 100)}%` }} />
        </div>
        <span className="telemetry-value">
          {kv == null ? "—" : `${Math.round(kv * 100)}%`}
        </span>
      </div>

      <div className="telemetry-stats">
        <div>
          <span className="telemetry-label">Generation</span>
          <span className="telemetry-value">{rate(telemetry?.genTps, telemetry?.lastGenTps, 1)}</span>
          <Sparkline values={history} />
        </div>
        <div>
          <span className="telemetry-label">Prompt eval</span>
          <span className="telemetry-value">
            {rate(telemetry?.promptTps, telemetry?.lastPromptTps, 0)}
          </span>
        </div>
        <div>
          <span className="telemetry-label">Tokens generated</span>
          <span className="telemetry-value">
            {telemetry?.tokensGenerated == null
              ? "—"
              : Math.round(telemetry.tokensGenerated).toLocaleString()}
          </span>
        </div>
        <div>
          <span className="telemetry-label">Tokens prompted</span>
          <span className="telemetry-value">
            {telemetry?.tokensPrompt == null
              ? "—"
              : Math.round(telemetry.tokensPrompt).toLocaleString()}
          </span>
        </div>
        <div>
          <span className="telemetry-label">Queue</span>
          <span className="telemetry-value">
            {telemetry?.requestsProcessing ?? 0} active
            {deferred > 0 && `, ${deferred} waiting`}
          </span>
        </div>
        <div>
          <span className="telemetry-label">System memory</span>
          <span className="telemetry-value">
            {telemetry?.systemUsedBytes && telemetry?.systemTotalBytes
              ? `${formatBytes(telemetry.systemUsedBytes)} of ${formatBytes(telemetry.systemTotalBytes)}`
              : "—"}
          </span>
          {telemetry?.swapUsedBytes ? (
            <span className="field-hint">
              swap {formatBytes(telemetry.swapUsedBytes)}
            </span>
          ) : null}
        </div>
      </div>

      {runner.serverCtx != null && (
        <p className="field-hint">
          server reports {runner.serverCtx.toLocaleString()} tokens of context
        </p>
      )}
    </div>
  );
}

export default function ModelDetail({
  model,
  runner,
  telemetry,
  logs,
  onBack,
  onRunnerChange,
}: {
  model: ModelEntry;
  runner: RunnerSnapshot;
  telemetry: Telemetry | null;
  logs: string[];
  onBack: () => void;
  onRunnerChange: (snapshot: RunnerSnapshot) => void;
}) {
  const [plan, setPlan] = useState<LaunchPlan | null>(null);
  const [form, setForm] = useState<Profile | null>(null);
  const [preview, setPreview] = useState<LaunchPlan | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [showLogs, setShowLogs] = useState(false);
  const history = useRef<number[]>([]);

  const isCurrent = runner.modelId === model.id;
  const running = isCurrent && (runner.state === "starting" || runner.state === "ready");

  useEffect(() => {
    getLaunchPlan(model.id)
      .then((next) => {
        setPlan(next);
        setPreview(next);
        setForm(next.profile);
      })
      .catch((e) => setFailure(String(e)));
  }, [model.id]);

  useEffect(() => {
    if (isCurrent && (runner.state === "starting" || runner.state === "crashed")) {
      setShowLogs(true);
    }
  }, [isCurrent, runner.state]);

  useEffect(() => {
    if (telemetry?.genTps == null) return;
    history.current = [...history.current, telemetry.genTps].slice(-SPARK_POINTS);
  }, [telemetry]);

  const patch = useMemo(() => {
    if (!form || !plan) return {};
    return diffProfile(form, plan.effectiveDefaults);
  }, [form, plan]);

  useEffect(() => {
    if (!form || !plan) return;
    const timer = setTimeout(() => {
      getLaunchPlan(model.id, patch).then(setPreview).catch(() => {});
    }, 200);
    return () => clearTimeout(timer);
  }, [patch, form, plan, model.id]);

  const guard = useCallback(async (action: () => Promise<RunnerSnapshot>) => {
    setBusy(true);
    setFailure(null);
    try {
      onRunnerChange(await action());
    } catch (e) {
      setFailure(String(e));
    } finally {
      setBusy(false);
    }
  }, [onRunnerChange]);

  const blocked = useMemo(() => {
    if (model.error) return "This file could not be parsed as GGUF.";
    if (model.shards && model.shards.missing.length > 0) {
      return "This shard set is incomplete.";
    }
    return preview?.capabilityError ?? null;
  }, [model, preview]);

  if (!plan || !form) {
    return (
      <>
        <header className="screen-header">
          <button className="button" onClick={onBack}>
            Back
          </button>
        </header>
        {failure && <p className="notice notice-error">{failure}</p>}
      </>
    );
  }

  return (
    <>
      <header className="screen-header">
        <div>
          <button className="link-back" onClick={onBack}>
            ← Library
          </button>
          <h1>{model.displayName}</h1>
          <p className="screen-subtitle">{model.path}</p>
        </div>
        <div className="actions">
          {running ? (
            <>
              <button
                className="button"
                disabled={busy}
                onClick={() => guard(() => runnerStart(model.id, patch))}
              >
                Reload
              </button>
              <button
                className="button button-danger"
                disabled={busy}
                onClick={() => guard(() => runnerStop())}
              >
                Stop
              </button>
            </>
          ) : (
            <button
              className="button button-primary"
              disabled={busy || blocked !== null}
              onClick={() => guard(() => runnerStart(model.id, patch))}
            >
              Run
            </button>
          )}
        </div>
      </header>

      {blocked && <p className="notice notice-error">{blocked}</p>}
      {failure && <p className="notice notice-error">{failure}</p>}

      {isCurrent && runner.state === "crashed" && (
        <div className="notice notice-error">
          <strong>{runner.error}</strong>
          {runner.crashTail.length > 0 && (
            <pre className="crash-tail">{runner.crashTail.join("\n")}</pre>
          )}
        </div>
      )}

      {isCurrent && runner.requestedPort != null &&
        runner.port != null &&
        runner.requestedPort !== runner.port && (
          <p className="notice">
            Port {runner.requestedPort} was busy — listening on {runner.port}.
          </p>
        )}

      {isCurrent && runner.state === "ready" && (
        <section className="panel">
          <h2>Running</h2>
          <TelemetryPanel
            runner={runner}
            telemetry={telemetry}
            history={history.current}
          />
        </section>
      )}

      <section className="panel">
        <h2>Model</h2>
        <Facts model={model} />
      </section>

      <section className="panel">
        <h2>Launch settings</h2>
        <ProfileForm
          value={form}
          defaults={plan.effectiveDefaults}
          maxCtx={plan.maxCtx}
          onChange={setForm}
        />
        <div className="panel-actions">
          <button
            className="button"
            disabled={Object.keys(patch).length === 0}
            onClick={() =>
              saveProfile(model.id, patch)
                .then((next) => {
                  setPlan(next);
                  setForm(next.profile);
                  setPreview(next);
                })
                .catch((e) => setFailure(String(e)))
            }
          >
            Save as this model's profile
          </button>
          {plan.overridden.length > 0 && (
            <span className="field-hint">
              saved overrides: {plan.overridden.join(", ")}
            </span>
          )}
        </div>
      </section>

      <section className="panel">
        <h2>Predicted memory</h2>
        <MemoryBar plan={preview ?? plan} />
      </section>

      <section className="panel">
        <h2>Command</h2>
        <pre className="command">{preview?.command ?? plan.command}</pre>
        <div className="panel-actions">
          <button
            className="button"
            onClick={() =>
              navigator.clipboard.writeText(preview?.command ?? plan.command)
            }
          >
            Copy
          </button>
        </div>
      </section>

      <section className="panel">
        <div className="panel-head">
          <h2>Logs</h2>
          <button className="button" onClick={() => setShowLogs((v) => !v)}>
            {showLogs ? "Hide" : "Show"}
          </button>
        </div>
        {showLogs && (
          <pre className="logs">
            {logs.length > 0 ? logs.join("\n") : "No output yet."}
          </pre>
        )}
      </section>
    </>
  );
}
