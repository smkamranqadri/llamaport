import { formatMemory } from "./format";
import { bytesOr, pressureText, Stat } from "./Memory";
import type { RunnerSnapshot, Telemetry } from "./types";

/// Samples a sparkline holds on the model screen; Activity keeps fewer.
export const SPARK_POINTS = 60;

function Sparkline({ values, capacity }: { values: number[]; capacity: number }) {
  if (values.length < 2) return null;
  const max = Math.max(...values, 1);
  const points = values
    .map((v, i) => `${(i / (capacity - 1)) * 100},${20 - (v / max) * 20}`)
    .join(" ");

  return (
    <svg className="spark" viewBox="0 0 100 20" preserveAspectRatio="none">
      <polyline points={points} />
    </svg>
  );
}

export function Card({
  label,
  value,
  sub,
  tone,
  subTone,
  fill,
  spark,
  capacity,
}: {
  label: string;
  value: string;
  sub?: string;
  tone?: "ok" | "bad";
  /// A verdict rather than a quantity, so it carries the colour the figure does not.
  subTone?: "ok" | "bad";
  /// 0 to 1, drawn as a bar under the figure.
  fill?: number | null;
  spark?: number[];
  capacity?: number;
}) {
  let figure = <span className={`card-big${tone ? ` tone-${tone}` : ""}`}>{value}</span>;
  if (spark && spark.length > 1) {
    figure = (
      <span className="card-figure">
        {figure}
        <Sparkline values={spark} capacity={capacity ?? SPARK_POINTS} />
      </span>
    );
  }

  return (
    <div className="card">
      <span className="card-label">{label}</span>
      {figure}
      {sub && (
        <span className={`card-sub${subTone ? ` tone-${subTone}` : ""}`}>{sub}</span>
      )}
      {fill != null && (
        <div className="bar">
          <span style={{ width: `${Math.min(100, Math.max(0, fill * 100))}%` }} />
        </div>
      )}
    </div>
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

export function TelemetryPanel({
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
          <Sparkline values={history} capacity={SPARK_POINTS} />
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
