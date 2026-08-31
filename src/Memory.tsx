import { formatMemory } from "./format";
import type { Pressure } from "./types";

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
