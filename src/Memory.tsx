import { formatMemory } from "./format";
import type { LaunchPlan, Pressure } from "./types";

const PRESSURE_LABEL: Record<Pressure, string> = {
  normal: "normal",
  warning: "elevated",
  critical: "critical",
  unknown: "Unavailable",
};

/// A missing reading is reported as missing, never as zero.
export function bytesOr(value: number | null | undefined): string {
  return value == null ? "Unavailable" : formatMemory(value);
}

export function pressureText(pressure: Pressure | null | undefined): string {
  if (!pressure) return "Unavailable";
  return PRESSURE_LABEL[pressure] ?? "Unavailable";
}

/// `tone` colours a value that carries a verdict rather than a quantity, so whether
/// something is wrong reads without being read.
export function Stat({
  label,
  value,
  hint,
  tone,
}: {
  label: string;
  value: string;
  hint?: string;
  tone?: "ok" | "warn" | "bad";
}) {
  const missing = value === "Unavailable";
  let className = "telemetry-value";
  if (missing) {
    className += " is-missing";
  }
  if (tone) {
    className += ` tone-${tone}`;
  }
  return (
    <div>
      <span className="telemetry-label">{label}</span>
      <span className={className}>{value}</span>
      {hint && <span className="field-hint">{hint}</span>}
    </div>
  );
}

/// What this launch asks of the machine, in one sentence. The bar and the folded row
/// both read it, so the summary can never disagree with the panel under it.
export function launchCost(
  plan: LaunchPlan,
): { wants: string; breakdown: string; verdict: string; tone: "ok" | "warn" | "bad" } | null {
  const estimate = plan.estimate;
  if (estimate == null) return null;

  const budget = plan.deviceBudgetBytes;
  const free = plan.memory.availableBytes;
  const strained =
    plan.memory.pressure === "warning" || plan.memory.pressure === "critical";
  const { weightsBytes, kvBytes, totalBytes, bounded } = estimate;
  const over = budget != null && totalBytes > budget;
  const crowded = free != null && totalBytes > free;

  let sign = "";
  if (bounded) {
    sign = "≥ ";
  }
  let wants = `${sign}${formatMemory(totalBytes)}`;
  let breakdown = `${formatMemory(weightsBytes)} weights + ${formatMemory(kvBytes)} cache at ${plan.profile.ctx.toLocaleString()} tokens`;
  if (kvBytes === 0) {
    wants = `${sign}${formatMemory(weightsBytes)}`;
    breakdown = "weights only — no context chosen, so the cache is not counted";
  }

  // Two questions, and the verdict is the worse of them. Fitting the GPU says a launch is
  // allowed; it says nothing about a machine with nothing left to give.
  let verdict = "fits";
  let tone: "ok" | "warn" | "bad" = "ok";
  if (over) {
    verdict = "over the GPU limit";
    tone = "bad";
  } else if (budget == null) {
    verdict = "ceiling unknown";
    tone = "warn";
  } else if (strained) {
    verdict = "fits, but the machine is under pressure";
    tone = "bad";
  } else if (crowded) {
    verdict = "fits the GPU, not what is free";
    tone = "warn";
  } else if (budget != null && totalBytes < budget * 0.6) {
    verdict = "fits comfortably";
  }

  return { wants, breakdown, verdict, tone };
}

export function MemoryBar({ plan }: { plan: LaunchPlan }) {
  const { memory } = plan;

  // Four figures, and the ceiling among them is the one the app used to get wrong: it
  // compared against installed memory, which nothing allocates from.
  const budget = plan.deviceBudgetBytes;
  const free = memory.availableBytes;
  const strained = memory.pressure === "warning" || memory.pressure === "critical";

  // Free memory is only good or bad relative to what is being asked of it, so the
  // figure cannot be coloured until the launch's size is known. Green beside a warning
  // that says the opposite is the bug this closes.
  const machineStats = (wants: number | null) => {
    let freeTone: "ok" | "warn" | "bad" | undefined;
    if (free != null) {
      freeTone = "ok";
      if (wants != null && free < wants) {
        freeTone = "warn";
      }
      if (strained || free < 1024 * 1024 * 1024) {
        freeTone = "bad";
      }
    }
    return (
      <div className="telemetry-stats">
        <Stat
          label="GPU limit"
          value={budget == null ? "Unknown" : formatMemory(budget)}
          hint={budget == null ? "this build does not report it" : "what a launch must fit inside"}
        />
        <Stat
          label="Free right now"
          value={bytesOr(free)}
          tone={freeTone}
          hint={`macOS pressure ${pressureText(memory.pressure)}`}
        />
        <Stat label="Swap in use" value={bytesOr(memory.swapUsedBytes)} hint="paging costs speed" />
        <Stat label="Installed" value={bytesOr(memory.installedBytes)} hint="the machine's spec" />
      </div>
    );
  };

  if (!plan.estimate) {
    return (
      <div className="memory">
        <p className="empty-detail">
          Not enough header metadata to size this model's cache.
        </p>
        {machineStats(null)}
      </div>
    );
  }

  const { weightsBytes, kvBytes, totalBytes, bounded, boundNote } = plan.estimate;
  // The bar is drawn against the ceiling a launch has to fit inside, falling back to
  // installed memory only where the build would not say what that ceiling is.
  const ceiling = budget ?? plan.totalMemory;
  const scale = Math.max(ceiling, totalBytes);
  const width = (n: number) => `${(n / scale) * 100}%`;

  const cost = launchCost(plan)!;

  return (
    <div className="memory">
      <div className="memory-bar">
        <span className="seg seg-weights" style={{ width: width(weightsBytes) }} />
        <span className="seg seg-kv" style={{ width: width(kvBytes) }} />
        <span className="memory-limit" style={{ left: `${(ceiling / scale) * 100}%` }} />
      </div>

      <div className="telemetry-stats">
        <Stat label="This launch wants" value={cost.wants} hint={cost.breakdown} />
        <Stat label="Against the limit" value={cost.verdict} tone={cost.tone} />
      </div>

      {bounded && boundNote && <p className="field-hint">{boundNote}</p>}

      {machineStats(totalBytes)}
    </div>
  );
}
