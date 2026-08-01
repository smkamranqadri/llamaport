import { formatBytes } from "./format";
import type { Assessment, Pressure, SafetyState } from "./types";

const STATE_LABEL: Record<SafetyState, string> = {
  unknown: "Unknown",
  green: "Comfortable",
  yellow: "Tight",
  red: "At risk",
};

const PRESSURE_LABEL: Record<Pressure, string> = {
  normal: "normal",
  warning: "elevated",
  critical: "critical",
  unknown: "Unavailable",
};

export function SafetyBadge({ state }: { state: SafetyState }) {
  return <span className={`badge safety-${state}`}>{STATE_LABEL[state]}</span>;
}

/// A missing reading is reported as missing, never as zero.
export function bytesOr(value: number | null | undefined): string {
  return value == null ? "Unavailable" : formatBytes(value);
}

export function headroomText(value: number | null | undefined): string {
  if (value == null) return "Unavailable";
  if (value < 0) return `${formatBytes(-value)} over`;
  return formatBytes(value);
}

export function pressureText(pressure: Pressure | null | undefined): string {
  if (!pressure) return "Unavailable";
  return PRESSURE_LABEL[pressure] ?? "Unavailable";
}

export function Reasons({ assessment }: { assessment: Assessment | null }) {
  if (!assessment || assessment.reasons.length === 0) return null;
  return <p className="memory-reasons">{assessment.reasons.join(" · ")}</p>;
}

export function Stat({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint?: string;
}) {
  const missing = value === "Unavailable";
  return (
    <div>
      <span className="telemetry-label">{label}</span>
      <span className={`telemetry-value${missing ? " is-missing" : ""}`}>
        {value}
      </span>
      {hint && <span className="field-hint">{hint}</span>}
    </div>
  );
}
