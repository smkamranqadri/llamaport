import type { CheckStatus, HealthReport, Verdict } from "./types";

const VERDICT_LABEL: Record<Verdict, string> = {
  passed: "Passed",
  passedWithWarnings: "Passed with warnings",
  failed: "Failed",
};

const VERDICT_CLASS: Record<Verdict, string> = {
  passed: "safety-green",
  passedWithWarnings: "safety-yellow",
  failed: "safety-red",
};

const STATUS_MARK: Record<CheckStatus, string> = {
  passed: "✓",
  warning: "!",
  failed: "✗",
  skipped: "–",
};

function tokens(value: number | null, unit: string) {
  if (value == null) return null;
  return `${value.toLocaleString()} ${unit}`;
}

function rate(value: number | null) {
  if (value == null) return null;
  return `${value.toFixed(1)} tok/s`;
}

export default function HealthPanel({ report }: { report: HealthReport }) {
  const { timings } = report;
  const summary = [
    timings.timeToFirstTokenMs != null &&
      `first token ${timings.timeToFirstTokenMs} ms`,
    timings.totalResponseMs != null && `total ${timings.totalResponseMs} ms`,
    tokens(timings.promptTokens, "prompt tokens"),
    tokens(timings.generatedTokens, "generated"),
    timings.promptTps != null && `prompt ${rate(timings.promptTps)}`,
    timings.genTps != null && `generation ${rate(timings.genTps)}`,
  ].filter(Boolean);

  return (
    <div className="health">
      <p className="memory-summary">
        <span className={`badge ${VERDICT_CLASS[report.verdict]}`}>
          {VERDICT_LABEL[report.verdict]}
        </span>
      </p>

      <ul className="check-list">
        {report.checks.map((check) => (
          <li key={check.name} className={`check check-${check.status}`}>
            <span className="check-mark">{STATUS_MARK[check.status]}</span>
            <span className="check-name">{check.name}</span>
            <span className="check-detail">{check.detail}</span>
            <span className="check-duration">{check.durationMs} ms</span>
          </li>
        ))}
      </ul>

      {summary.length > 0 && (
        <p className="field-hint">{summary.join(" · ")}</p>
      )}
    </div>
  );
}
