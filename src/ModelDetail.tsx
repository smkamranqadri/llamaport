import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  getLaunchPlan,
  healthTest,
  openWebUi,
  revealPath,
  runnerStart,
  runnerStop,
} from "./api";
import { formatContext, formatFileSize, formatMemory } from "./format";
import { bytesOr, pressureText, Stat } from "./Memory";
import ProfileForm from "./ProfileForm";
import HealthPanel from "./HealthPanel";
import type {
  HealthReport,
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
    ["Size", formatFileSize(model.sizeBytes)],
    ["Architecture", md?.architecture ?? "unknown"],
    ["Quantisation", model.quant ?? "unknown"],
    ["Parameters", md?.sizeLabel ?? "—"],
    ["Max context", md?.contextLength ? formatContext(md.contextLength) : "—"],
    ["Layers", md?.blockCount?.toString() ?? "—"],
    ["KV heads", md?.headCountKv?.toString() ?? "—"],
    ["Chat template", md?.hasChatTemplate ? "embedded" : "none — prompts may be misformatted"],
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

/// Both branches of the memory panel say this, because both print a weights figure the
/// Model panel prints differently, and the branch that withholds the cache is the one a
/// reader is most likely to be studying when they notice.
const COUNTED_AS_MEMORY =
  "Counted as the machine counts memory — the Model panel's file size is these same bytes counted as Finder counts them, so it reads larger.";

/// The cache is a figure or a reason there is none; it is never a guess.
function kvStat(plan: LaunchPlan): { value: string; hint: string } {
  const kv = plan.estimate?.kvBytes;
  if (kv == null) {
    return {
      value: "Unavailable",
      hint: "the header does not size these layers",
    };
  }
  return { value: formatMemory(kv), hint: "from the model's header" };
}

function MemoryBar({ plan }: { plan: LaunchPlan }) {
  const { memory } = plan;

  const machine = (
    <div className="telemetry-stats">
      <Stat label="Installed" value={bytesOr(memory.installedBytes)} />
      <Stat label="In use now" value={bytesOr(memory.usedBytes)} />
      <Stat label="Swap in use" value={bytesOr(memory.swapUsedBytes)} />
      <Stat label="macOS pressure" value={pressureText(memory.pressure)} />
    </div>
  );

  if (!plan.estimate) {
    return (
      <div className="memory">
        <p className="empty-detail">
          Not enough header metadata to size this model's cache.
        </p>
        {machine}
      </div>
    );
  }

  const { weightsBytes, kvBytes, totalBytes, kvUnknown } = plan.estimate;
  const scale = Math.max(plan.totalMemory, totalBytes ?? weightsBytes);
  const width = (n: number) => `${(n / scale) * 100}%`;

  const bar = (
    <div className="memory-bar">
      <span className="seg seg-weights" style={{ width: width(weightsBytes) }} />
      {kvBytes != null && (
        <span className="seg seg-kv" style={{ width: width(kvBytes) }} />
      )}
      <span
        className="memory-limit"
        style={{ left: `${(plan.totalMemory / scale) * 100}%` }}
      />
    </div>
  );

  if (kvBytes == null || totalBytes == null) {
    return (
      <div className="memory">
        {bar}
        <p className="memory-summary">
          <strong>{formatMemory(weightsBytes)}</strong> of weights. What the cache
          holds at {plan.profile.ctx.toLocaleString()} tokens is not shown.
        </p>
        <p className="field-hint">
          {kvUnknown} {COUNTED_AS_MEMORY}
        </p>
        {machine}
      </div>
    );
  }

  return (
    <div className="memory">
      {bar}

      <p className="memory-summary">
        <strong>{formatMemory(totalBytes)}</strong> to allocate — weights{" "}
        {formatMemory(weightsBytes)} plus {formatMemory(kvBytes)} of KV cache at{" "}
        {plan.profile.ctx.toLocaleString()} tokens.
      </p>
      <p className="field-hint">
        Exact figures from the model's header. {COUNTED_AS_MEMORY} How much of it
        stays resident, and what that costs the machine, depends on what else is
        running — the numbers below are the machine as it is now.
      </p>

      {machine}
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
        <Stat
          label="System memory"
          value={
            telemetry?.systemUsedBytes != null &&
            telemetry?.systemTotalBytes != null
              ? `${formatMemory(telemetry.systemUsedBytes)} of ${formatMemory(telemetry.systemTotalBytes)}`
              : "Unavailable"
          }
        />
        <Stat
          label="macOS pressure"
          value={pressureText(telemetry?.pressure)}
        />
        <Stat label="Swap in use" value={bytesOr(telemetry?.swapUsedBytes)} />
        <Stat
          label="Process footprint"
          value={bytesOr(telemetry?.processFootprintBytes)}
          hint="excludes GPU-resident weights"
        />
      </div>


      <div className="telemetry-row">
        <span className="telemetry-label">Health</span>
        <span className="telemetry-value">
          <span className={`dot state-${telemetry?.healthOk ? "ready" : "starting"}`} />
          {telemetry?.healthOk
            ? " responding now"
            : " process alive, endpoint not answering"}
        </span>
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
  const [health, setHealth] = useState<HealthReport | null>(null);
  const [testing, setTesting] = useState(false);
  const history = useRef<number[]>([]);

  const isCurrent = runner.modelId === model.id;
  const running = isCurrent && (runner.state === "starting" || runner.state === "ready");
  const { port } = runner;

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
    // A report describes one run; keeping it visible across a restart would misreport.
    if (runner.state !== "ready") {
      setHealth(null);
    }
  }, [isCurrent, runner.state]);

  useEffect(() => {
    if (telemetry?.genTps == null) return;
    history.current = [...history.current, telemetry.genTps].slice(-SPARK_POINTS);
  }, [telemetry]);

  // The form is the launch: there is no saved profile to diff against.
  const draft = form ?? undefined;

  useEffect(() => {
    if (!form || !plan) return;
    const timer = setTimeout(() => {
      getLaunchPlan(model.id, draft).then(setPreview).catch(() => {});
    }, 200);
    return () => clearTimeout(timer);
  }, [draft, form, plan, model.id]);

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
          <h1 title={model.fileName}>{model.displayName}</h1>
          <p className="screen-subtitle" title={model.path}>
            {model.path}{" "}
            <button
              className="link-stop"
              onClick={() => void revealPath(model.path).catch(() => {})}
            >
              reveal in Finder
            </button>
          </p>
        </div>
        <div className="actions">
          {running ? (
            <>
              <button
                className="button"
                disabled={busy}
                onClick={() => guard(() => runnerStart(model.id, draft))}
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
              onClick={() => guard(() => runnerStart(model.id, draft))}
            >
              Run
            </button>
          )}
        </div>
      </header>

      {blocked && <p className="notice notice-error">{blocked}</p>}

      {!running && (preview ?? plan).portConflict && (
        <p className="notice">
          Port {(preview ?? plan).portConflict!.port} is already in use
          {(preview ?? plan).portConflict!.isLlamaServer
            ? " by another llama-server — probably one this app lost track of."
            : " by another process."}{" "}
          This launch will be refused rather than moved to another port — free{" "}
          {(preview ?? plan).portConflict!.port} and try again.
        </p>
      )}
      {failure && <p className="notice notice-error">{failure}</p>}

      {isCurrent && runner.state === "crashed" && (
        <div className="notice notice-error">
          <strong>{runner.error}</strong>
          {runner.crashTail.length > 0 && (
            <pre className="crash-tail">{runner.crashTail.join("\n")}</pre>
          )}
        </div>
      )}


      {isCurrent && runner.state === "ready" && (
        <section className="panel">
          <div className="panel-head">
            <h2>Running</h2>
            <span className="actions">
              {port != null && (
                <button
                  className="button"
                  onClick={() => {
                    setFailure(null);
                    openWebUi(port).catch((e) => setFailure(String(e)));
                  }}
                >
                  Web UI
                </button>
              )}
              <button
                className="button"
                disabled={testing}
                onClick={() => {
                  setTesting(true);
                  setFailure(null);
                  healthTest()
                    .then(setHealth)
                    .catch((e) => setFailure(String(e)))
                    .finally(() => setTesting(false));
                }}
              >
                {testing ? "Testing…" : "Test model"}
              </button>
            </span>
          </div>
          <TelemetryPanel
            runner={runner}
            telemetry={telemetry}
            history={history.current}
          />
          {health && <HealthPanel report={health} />}
        </section>
      )}

      <section className="panel">
        <h2>Model</h2>
        <Facts model={model} />
      </section>

      <section className="panel">
        <h2>Context</h2>
        <div className="telemetry-stats">
          <Stat
            label="Model maximum"
            value={
              plan.maxCtx == null ? "Unavailable" : plan.maxCtx.toLocaleString()
            }
            hint="from the file's metadata"
          />
          <Stat
            label="Current profile"
            value={form.ctx.toLocaleString()}
            hint="what this launch will request"
          />
          <Stat
            label="KV cache at this context"
            value={kvStat(plan).value}
            hint={kvStat(plan).hint}
          />
        </div>
      </section>

      <section className="panel">
        <h2>Launch settings</h2>

        <ProfileForm
          value={form}
          maxCtx={plan.maxCtx}
          onChange={setForm}
        />

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
