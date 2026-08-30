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
