import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  getLaunchPlan,
  getSettings,
  healthTest,
  onTuneReport,
  openWebUi,
  revealPath,
  runnerStart,
  runnerStop,
  speedsFor,
  tuneCancel,
  tuneStart,
  tuneStatus,
} from "./api";
import { formatContext, formatFileSize, formatMemory } from "./format";
import { bytesOr, launchCost, MemoryBar } from "./Memory";
import { CloseIcon, CopyIcon, PiIcon, PlayIcon, StopIcon } from "./icons";
import Disclosure from "./Disclosure";
import { AdvancedFields, AUTO_CTX, ProfileFields } from "./ProfileForm";
import HealthPanel from "./HealthPanel";
import PiPanel from "./PiPanel";
import Presets, {
  presetName,
  selectedPreset,
  speedFields,
  type Which,
} from "./Presets";
import { Card, SPARK_POINTS, TelemetryPanel } from "./Telemetry";
import TunePanel from "./TunePanel";
import type {
  HealthReport,
  LaunchPlan,
  ModelEntry,
  Profile,
  RunnerSnapshot,
  SpeedSummary,
  Telemetry,
  TuneReport,
} from "./types";

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

export default function ModelDetail({
  model,
  runner,
  telemetry,
  logs,
  onRunnerChange,
}: {
  model: ModelEntry;
  runner: RunnerSnapshot;
  telemetry: Telemetry | null;
  logs: string[];
  onRunnerChange: (snapshot: RunnerSnapshot) => void;
}) {
  const [plan, setPlan] = useState<LaunchPlan | null>(null);
  const [form, setForm] = useState<Profile | null>(null);
  const [preview, setPreview] = useState<LaunchPlan | null>(null);
  /// A plan priced at the context the server actually fitted. Only the memory panel
  /// reads it — the command display must keep showing the argv that was launched, which
  /// names no context at all.
  const [fitted, setFitted] = useState<LaunchPlan | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [showLogs, setShowLogs] = useState(false);
  const [health, setHealth] = useState<HealthReport | null>(null);
  const [pi, setPi] = useState(false);
  const [testing, setTesting] = useState(false);
  const [tune, setTune] = useState<TuneReport | null>(null);
  const [speeds, setSpeeds] = useState<SpeedSummary | null>(null);
  const [speedOpen, setSpeedOpen] = useState(false);
  const wasMeasuring = useRef(false);
  const [defaults, setDefaults] = useState<Profile | null>(null);
  /// The preset card last pressed. Cleared by any other edit, so a hand-changed field
  /// falls back to whatever the values themselves say.
  const [picked, setPicked] = useState<Which>(null);
  const history = useRef<number[]>([]);

  const mine = tune != null && tune.modelId === model.id ? tune : null;
  const measuring = mine?.running === true;

  const isCurrent = runner.modelId === model.id;
  const running = isCurrent && (runner.state === "starting" || runner.state === "ready");
  const crashed = isCurrent && runner.state === "crashed";
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
    if (!running || form?.ctx !== AUTO_CTX || runner.serverCtx == null) {
      setFitted(null);
      return;
    }
    getLaunchPlan(model.id, { ...form, ctx: runner.serverCtx })
      .then(setFitted)
      .catch(() => {});
  }, [running, runner.serverCtx, model.id, form]);

  useEffect(() => {
    // Only a crash opens the log by itself. A launch that works has nothing in it the
    // reader wanted, and unrolling a thousand lines under four calm figures undoes them.
    if (isCurrent && runner.state === "crashed") {
      setShowLogs(true);
    }
    // A report describes one run; keeping it visible across a restart would misreport.
    if (runner.state !== "ready") {
      setHealth(null);
    }
  }, [isCurrent, runner.state]);

  useEffect(() => {
    tuneStatus().then(setTune).catch(() => {});
    getSettings()
      .then((s) => setDefaults(s.launchDefaults ?? s.builtInDefaults))
      .catch(() => {});
    const stop = onTuneReport(setTune);
    return () => {
      void stop.then((off) => off());
    };
  }, []);

  // Rows are written when a run settles and as each candidate finishes, so the history is
  // re-read on both rather than only when the screen opens.
  useEffect(() => {
    speedsFor(model.id).then(setSpeeds).catch(() => {});
  }, [model.id, runner.state, tune?.done, tune?.running]);

  // The ladder opens its own row and closes it again when it is over: what it was asked
  // for is one preset, and reading four tries afterwards is a choice, not the outcome.
  useEffect(() => {
    if (measuring) {
      wasMeasuring.current = true;
      setSpeedOpen(true);
      return;
    }
    if (wasMeasuring.current) {
      wasMeasuring.current = false;
      setSpeedOpen(false);
      // Read the history again here rather than waiting for the effect that watches the
      // report: the suggestion this applies has to be the one the last row produced, and
      // the copy in state is still the one from before the ladder ran.
      speedsFor(model.id)
        .then((next) => {
          setSpeeds(next);
          const key = next.suggestion;
          if (key == null) return;
          setPicked("best");
          setForm((current) => current && { ...current, ...speedFields(key) });
        })
        .catch(() => {});
    }
  }, [measuring, model.id]);

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
          <h1>{model.displayName}</h1>
        </header>
        {failure && <p className="notice notice-error">{failure}</p>}
      </>
    );
  }

  // One source for every figure on this screen: the fitted plan while a model runs under
  // Auto, else the debounced preview of what the form holds, else the plan as loaded.
  // Reading some figures from one and some from another put a cache priced at 32,768
  // beside a profile reading 0.
  const shown = fitted ?? preview ?? plan;

  const tuneBlocked =
    blocked ??
    (runner.state === "idle"
      ? null
      : "Tune launches servers of its own, and this app runs one model at a time. Stop the running model first.");

  const cost = launchCost(shown);
  let ceilingText = "";
  if (shown.deviceBudgetBytes != null) {
    ceilingText = ` of ${formatMemory(shown.deviceBudgetBytes)}`;
  }
  const preset = presetName(
    picked ??
      selectedPreset({
        form,
        defaults,
        summary: speeds,
        fitAvailable: plan.fitAvailable,
      }),
  );
  /// Every route to the form other than a preset card, which is what makes the highlight
  /// honest: touch a field and the cards stop claiming credit for the values.
  const editForm = (next: Profile) => {
    setPicked(null);
    setForm(next);
  };
  const applyPreset = (partial: Partial<Profile>, which: Which) => {
    setPicked(which);
    setForm((current) => current && { ...current, ...partial });
  };
  let advancedSub = "";
  if (preset) {
    advancedSub = ` — using ${preset} values`;
  }
  let commandSub = "see exactly what will run before you press Run";
  if (running) {
    commandSub = "the exact llama-server command this run uses";
  }
  // The four figures the running screen leads with, each said the way someone who is not
  // an expert would read it, with the number they would quote kept beside it.
  const totalMem = telemetry?.systemTotalBytes ?? null;
  let memoryCardSub = "what this model is using right now";
  let memoryFill: number | null = null;
  if (totalMem != null) {
    let verdict = "plenty left";
    const used = telemetry?.systemUsedBytes;
    if (used != null) {
      const freeShare = 1 - used / totalMem;
      if (freeShare < 0.15) verdict = "little left";
      else if (freeShare < 0.35) verdict = "some room left";
    }
    if (telemetry?.pressure === "warning" || telemetry?.pressure === "critical") {
      verdict = "the Mac is under pressure";
    }
    memoryCardSub = `of ${formatMemory(totalMem)} — ${verdict}`;
    if (telemetry?.processFootprintBytes != null) {
      memoryFill = telemetry.processFootprintBytes / totalMem;
    }
  }

  const gen = telemetry?.genTps ?? telemetry?.lastGenTps ?? null;
  const prompt = telemetry?.promptTps ?? telemetry?.lastPromptTps ?? null;
  let speedValue = "—";
  if (gen != null && gen > 0) {
    speedValue = `${gen.toFixed(0)} tok/s`;
  }
  let speedSubtitle = "writing speed — ask it something to see one";
  if (prompt != null && prompt > 0) {
    speedSubtitle = `writing speed · reads your prompt at ${prompt.toFixed(0)} tok/s`;
  }

  const kv = telemetry?.kvCacheUsage ?? null;
  let contextValue = "—";
  if (kv != null) {
    contextValue = `${Math.round(kv * 100)}% full`;
  }
  let contextSubtitle = "how much of the conversation it is holding";
  if (runner.serverCtx != null) {
    const used = kv == null ? null : Math.round(kv * runner.serverCtx);
    contextSubtitle =
      used == null
        ? `${runner.serverCtx.toLocaleString()} tokens available`
        : `${used.toLocaleString()} of ${runner.serverCtx.toLocaleString()} tokens in use`;
  }

  let healthValue = "Starting…";
  let healthTone: "ok" | "bad" | undefined;
  let healthSubtitle = "waiting for the server to answer";
  if (runner.state === "ready") {
    if (telemetry?.healthOk === false) {
      healthValue = "Not answering";
      healthTone = "bad";
      healthSubtitle = "the process is alive but the endpoint is silent";
    } else {
      healthValue = "Ready";
      healthTone = "ok";
      healthSubtitle = "answering requests now";
    }
  }
  const passed = health?.checks.filter((c) => c.status === "passed").length ?? 0;
  if (health) {
    const first = health.timings.timeToFirstTokenMs;
    healthSubtitle = `${passed} of ${health.checks.length} checks passed`;
    if (first != null) {
      healthSubtitle += ` · first reply in ${(first / 1000).toFixed(1)} s`;
    }
    if (health.verdict === "failed") {
      healthValue = "Problem";
      healthTone = "bad";
    }
  }

  let testSub = "not run yet";
  let testDot: "ok" | "warn" | "bad" | undefined;
  if (health) {
    testSub = `${passed} of ${health.checks.length} checks passed`;
    testDot = "ok";
    if (health.verdict === "passedWithWarnings") testDot = "warn";
    if (health.verdict === "failed") {
      testDot = "bad";
      testSub = `${health.checks.length - passed} of ${health.checks.length} checks did not pass`;
    }
  }

  // The row is folded once the ladder is over, so its one line has to carry the outcome.
  let speedTitle = "Measuring best speed";
  let speedSub = "trying safe combinations of context and memory precision";
  if (mine != null && mine.total > 0) {
    speedSub = `${mine.done} of ${mine.total} tries done`;
    if (!mine.running) {
      speedTitle = "Best speed measured";
      speedSub = `${mine.done} tries measured`;
      if (speeds?.suggestion != null) {
        speedSub += " · Best speed selected";
      }
    }
    if (mine.cancelled) {
      speedTitle = "Measurement cancelled";
      speedSub = `${mine.done} of ${mine.total} tries measured`;
    }
  }

  return (
    <>
      <header className="screen-header">
        <div>
          <span className="title-row">
            <h1 title={model.fileName}>{model.displayName}</h1>
            {isCurrent && runner.state === "ready" && (
              <span className="pill pill-running">
                <span className="dot tone-ok" />
                Running
              </span>
            )}
            {isCurrent && runner.state === "starting" && (
              <span className="pill pill-starting">
                <span className="dot state-starting" />
                Starting…
              </span>
            )}
          </span>
          {!running && (
            <div className="badges-row">
              {model.quant && <span className="badge">{model.quant}</span>}
              {model.metadata?.sizeLabel && (
                <span className="badge">{model.metadata.sizeLabel}</span>
              )}
              {model.metadata?.contextLength != null && (
                <span className="badge">
                  {formatContext(model.metadata.contextLength)} context
                </span>
              )}
              <span className="badge">{formatFileSize(model.sizeBytes)}</span>
              {Boolean(model.metadata?.expertCount) && (
                <span className="badge badge-moe">MoE</span>
              )}
              {model.metadata != null && !model.metadata.hasChatTemplate && (
                <span className="badge badge-warn">no template</span>
              )}
            </div>
          )}
        </div>
        <div className="actions">
          {running ? (
            <>
              {port != null && runner.state === "ready" && (
                <button
                  className="button"
                  onClick={() => {
                    setFailure(null);
                    openWebUi(port).catch((e) => setFailure(String(e)));
                  }}
                >
                  Open chat
                </button>
              )}
              <button
                className="button"
                disabled={busy}
                onClick={() => guard(() => runnerStart(model.id, draft))}
              >
                Reload
              </button>
              {runner.state === "ready" && (
                <button
                  className="button button-primary"
                  onClick={() => setPi(true)}
                >
                  <PiIcon />
                  Use in pi
                </button>
              )}
              <button
                className="button button-danger"
                disabled={busy}
                onClick={() => guard(() => runnerStop())}
              >
                <StopIcon />
                Stop
              </button>
            </>
          ) : (
            <button
              className="button button-primary button-lg"
              disabled={busy || blocked !== null}
              onClick={() => guard(() => runnerStart(model.id, draft))}
            >
              <PlayIcon />
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

      {crashed && (
        <div className="notice notice-error">
          <strong>{runner.error}</strong>
          {runner.crashTail.length > 0 && (
            <pre className="crash-tail">{runner.crashTail.join("\n")}</pre>
          )}
        </div>
      )}


      {isCurrent && runner.state === "ready" && (
        <>
          <div className="cards">
            <Card
              label="Memory"
              value={bytesOr(telemetry?.processFootprintBytes)}
              sub={memoryCardSub}
              fill={memoryFill}
            />
            <Card label="Speed" value={speedValue} sub={speedSubtitle} />
            <Card
              label="Context"
              value={contextValue}
              sub={contextSubtitle}
              fill={telemetry?.kvCacheUsage ?? null}
            />
            <Card
              label="Health"
              value={healthValue}
              tone={healthTone}
              sub={healthSubtitle}
            />
          </div>

          {port != null && (
            <p className="address">
              <span>Other apps reach this model at</span>
              <code>http://localhost:{port}/v1</code>
              <button
                className="button"
                onClick={() =>
                  navigator.clipboard.writeText(`http://localhost:${port}/v1`)
                }
              >
                <CopyIcon />
                Copy
              </button>
            </p>
          )}

          {pi && <PiPanel onClose={() => setPi(false)} />}
        </>
      )}

      {/* A running model has already answered "how should it run"; the choice folds away
          under Details, where the mockup puts it, and the figures take the top. */}
      {running ? (
        <h2 className="group-label">Details</h2>
      ) : (
        <>
          <h2 className="group-label">How should it run?</h2>

          <Presets
            form={form}
            defaults={defaults}
            summary={speeds}
            fitAvailable={plan.fitAvailable}
            measureBlocked={tuneBlocked}
            measuring={measuring}
            picked={picked}
            onApply={applyPreset}
            onMeasure={() => {
              setFailure(null);
              tuneStart(model.id)
                .then(setTune)
                .catch((e) => setFailure(String(e)));
            }}
          />

          <ProfileFields
            value={form}
            maxCtx={plan.maxCtx}
            fitAvailable={plan.fitAvailable}
            onChange={editForm}
          />

          <Disclosure
            flat
            dot={cost?.tone ?? "warn"}
            title={
              cost
                ? `Estimated memory ${cost.wants}${ceilingText} — ${cost.verdict}`
                : "Memory — not enough header metadata to size this launch"
            }
          >
            <MemoryBar plan={shown} />
          </Disclosure>
        </>
      )}

      <div className="disclosures">
        {running && (
          <Disclosure
            title="Test results"
            sub={testSub}
            dot={testDot}
            action={
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
                {testing ? "Testing…" : "Test again"}
              </button>
            }
          >
            {health ? (
              <HealthPanel report={health} />
            ) : (
              <p className="field-hint">
                Nothing has been checked yet. Test again runs every check —
                process, port, health endpoint, model list, alias, and a real
                reply — and says what passed.
              </p>
            )}
          </Disclosure>
        )}

        {running && runner.state === "ready" && (
          <Disclosure
            title="Live details"
            sub="token counts, queue, swap and the rate over time"
          >
            <TelemetryPanel
              runner={runner}
              telemetry={telemetry}
              history={history.current}
            />
          </Disclosure>
        )}

        {!running && (
          <Disclosure
            title="Advanced"
            sub={`GPU layers, cache types, parallel slots, flags${advancedSub}`}
          >
            <AdvancedFields value={form} onChange={editForm} />
          </Disclosure>
        )}

        <Disclosure title="Full command" sub={commandSub}>
            <pre className="command">{preview?.command ?? plan.command}</pre>
            <div className="panel-actions">
              <button
                className="button"
                onClick={() =>
                  navigator.clipboard.writeText(preview?.command ?? plan.command)
                }
              >
                <CopyIcon />
                Copy
              </button>
            </div>
        </Disclosure>

        <Disclosure title="Model details" sub={model.fileName}>
          <Facts model={model} />
          <div className="panel-actions">
            <button
              className="button"
              onClick={() => void revealPath(model.path).catch(() => {})}
            >
              Reveal in Finder
            </button>
          </div>
        </Disclosure>

        {/* The ladder while it runs and while its answer is still on screen: a calm
            stopped model's history is the one line the Best speed card carries. Cancel
            sits in the summary, where the artboard's header puts it. */}
        {!running && mine != null && (mine.running || mine.rows.length > 0) ? (
          <Disclosure
            title={speedTitle}
            sub={speedSub}
            open={speedOpen}
            onToggle={setSpeedOpen}
            action={
              mine.running && (
                <button
                  className="button button-plain button-danger"
                  onClick={() => {
                    setFailure(null);
                    tuneCancel()
                      .then(setTune)
                      .catch((e) => setFailure(String(e)));
                  }}
                >
                  <CloseIcon />
                  Cancel
                </button>
              )
            }
          >
            <TunePanel
              report={mine}
              summary={speeds}
              onApply={(settings) => {
                setForm((current) => current && { ...current, ...settings });
                setSpeedOpen(false);
              }}
            />
          </Disclosure>
        ) : null}

        {(running || crashed) && logs.length > 0 && (
          <Disclosure
            title="Logs"
            sub="output from llama-server"
            open={showLogs}
            onToggle={setShowLogs}
          >
            <pre className="logs">{logs.join("\n")}</pre>
          </Disclosure>
        )}
      </div>
    </>
  );
}
