import { useState } from "react";
import { tuneCancel, tuneStart } from "./api";
import type {
  Profile,
  SpeedKey,
  SpeedRow,
  SpeedSummary,
  TuneOutcome,
  TuneReport,
} from "./types";

function tps(outcome: TuneOutcome, of: "gen" | "prompt"): number | null {
  const reading = outcome.reading;
  if (reading == null) return null;
  const tokens = of === "gen" ? reading.genTokens : reading.promptTokens;
  const seconds = of === "gen" ? reading.genSeconds : reading.promptSeconds;
  if (seconds <= 0) return null;
  return tokens / seconds;
}

function describe(ctx: number, cache: string) {
  return `${ctx.toLocaleString()} · ${cache}`;
}

/// What the ladder found, said in one line.
///
/// The first candidate is the one arithmetic picks — the largest context and most
/// precise cache that fits. Tune exists because that is not reliably the fastest, so the
/// comparison worth printing is against that row rather than against the app's default.
function Verdict({ rows }: { rows: TuneOutcome[] }) {
  const timed = rows.filter((row) => tps(row, "gen") != null);
  if (timed.length < 2) return null;

  const fastest = timed.reduce((best, row) =>
    tps(row, "gen")! > tps(best, "gen")! ? row : best,
  );
  const arithmetic = timed[0];
  const best = tps(fastest, "gen")!;
  const chosen = tps(arithmetic, "gen")!;

  if (fastest === arithmetic) {
    return (
      <p className="field-hint">
        The largest that fits was also the fastest — nothing was traded.
      </p>
    );
  }

  const faster = Math.round((best / chosen - 1) * 100);
  return (
    <p className="field-hint">
      <strong>
        {describe(fastest.candidate.ctx, fastest.candidate.cacheK)} is {faster}%
        faster
      </strong>{" "}
      than {describe(arithmetic.candidate.ctx, arithmetic.candidate.cacheK)}, the
      largest that fits. Arithmetic would have chosen the slower one.
    </p>
  );
}

/// What a row's figure is worth. A rate is only comparable to another rate that did the
/// same amount of work, so the workload travels with it rather than sitting in a sentence
/// underneath.
function Workload({ row }: { row: SpeedRow }) {
  const when = new Date(row.timestampSecs * 1000).toLocaleDateString();
  const runs = row.runs > 1 ? `best of ${row.runs} · ` : "";
  if (row.stale) {
    return (
      <span className="tune-note">
        {runs}build {row.llamaVersion ?? "unknown"} — not compared with this one
      </span>
    );
  }
  if (!row.ranked) {
    return (
      <span className="tune-note">
        {runs}{Math.round(row.promptTokens).toLocaleString()}-token prompt,{" "}
        {Math.round(row.genTokens).toLocaleString()} generated — too little work
        to rank
      </span>
    );
  }
  return (
    <span className="tune-note">
      {runs}
      {row.source === "measured" ? "tuned" : "in use"} {when} ·{" "}
      {Math.round(row.promptTokens).toLocaleString()}-token prompt
    </span>
  );
}

function keyText(key: SpeedKey) {
  return `${key.ctx.toLocaleString()} · ${key.cacheTypeK}`;
}

/// Two bare rates in a row say nothing about which is which.
function Head() {
  return (
    <div className="tune-row tune-head">
      <span>Context · cache</span>
      <span>Generation</span>
      <span>Prompt eval</span>
    </div>
  );
}

function sameKey(a: SpeedKey, b: SpeedKey) {
  return a.ctx === b.ctx && a.cacheTypeK === b.cacheTypeK && a.cacheTypeV === b.cacheTypeV;
}

/// The app's one opinion, and how much weight it carries.
function Suggestion({
  summary,
  onApply,
}: {
  summary: SpeedSummary;
  onApply: (key: SpeedKey) => void;
}) {
  if (summary.suggestion == null) {
    return (
      <p className="empty-detail">
        No opinion yet — nothing has done enough work to be worth ranking.
      </p>
    );
  }

  const key = summary.suggestion;
  const tps = summary.suggestedTps;
  return (
    <div className="tune-suggestion">
      <div className="telemetry-stats">
        <div>
          <span className="telemetry-label">Suggested</span>
          <span className="telemetry-value">{keyText(key)}</span>
        </div>
        <div>
          <span className="telemetry-label">Generation</span>
          <span className="telemetry-value">
            {tps == null ? "—" : `${tps.toFixed(1)} tok/s`}
          </span>
        </div>
        <button className="button" onClick={() => onApply(key)}>
          Use these settings
        </button>
      </div>

      {summary.tied > 1 && (
        <p className="field-hint">
          {summary.tied} settings measured within 10% of each other, which is
          inside what repeated runs of the same pair disagree by. The widest
          context among them is suggested rather than the fastest reading —
          context costs nothing when the speed is the same.
        </p>
      )}
      {summary.beats && summary.beatsByPercent != null && (
        <p className="field-hint">
          {Math.round(summary.beatsByPercent)}% faster than{" "}
          {keyText(summary.beats)}, which is what fitting the largest context
          would have chosen.
        </p>
      )}
      {summary.confidence === "observed" && (
        <p className="field-hint">
          From ordinary use, not a measurement: those runs answered different
          questions, so the ordering can be wrong. Tune asks every setting the
          same one.
        </p>
      )}
    </div>
  );
}

export default function TunePanel({
  modelId,
  blocked,
  report,
  onReport,
  summary,
  onApply,
}: {
  modelId: string;
  /// Why Tune cannot run right now, or null. It launches servers of its own.
  blocked: string | null;
  report: TuneReport | null;
  onReport: (report: TuneReport) => void;
  summary: SpeedSummary | null;
  onApply: (settings: Partial<Profile>) => void;
}) {
  const [failure, setFailure] = useState<string | null>(null);

  const mine = report != null && report.modelId === modelId;
  const running = report?.running === true;
  const rows = mine ? report.rows : [];

  const fastest = rows
    .filter((row) => tps(row, "gen") != null)
    .reduce<TuneOutcome | null>(
      (best, row) => (best == null || tps(row, "gen")! > tps(best, "gen")! ? row : best),
      null,
    );

  const act = (action: () => Promise<TuneReport>) => {
    setFailure(null);
    action().then(onReport).catch((e) => setFailure(String(e)));
  };

  return (
    <div>
      <div className="panel-head">
        <span className="field-hint">
          Launches this model at a few settings, asks each the same question,
          and times the answer.
        </span>
        <span className="actions">
          {running ? (
            <button className="button" onClick={() => act(tuneCancel)}>
              Cancel
            </button>
          ) : (
            <button
              className="button"
              disabled={blocked !== null}
              title={blocked ?? undefined}
              onClick={() => act(() => tuneStart(modelId))}
            >
              Tune
            </button>
          )}
        </span>
      </div>

      {failure && <p className="notice notice-error">{failure}</p>}
      {blocked && !running && <p className="field-hint">{blocked}</p>}

      {summary && (
        <Suggestion
          summary={summary}
          onApply={(key) =>
            onApply({
              ctx: key.ctx,
              cacheTypeK: key.cacheTypeK,
              cacheTypeV: key.cacheTypeV,
              ngl: key.ngl,
              flashAttn: key.flashAttn,
              parallel: key.parallel,
            })
          }
        />
      )}

      {rows.length === 0 && !running && summary?.suggestion == null && (
        <p className="field-hint">
          Use the model and it will say what it got; press Tune to measure. A
          memory sum says a launch is allowed, never that it is fast.
        </p>
      )}

      {mine && running && (
        <p className="field-hint">
          Measuring {report.done + 1} of {report.total}
          {report.current &&
            ` — ${describe(report.current.ctx, report.current.cacheK)}`}
          . Each launch loads the whole model, so this takes a few minutes.
        </p>
      )}

      {rows.length > 0 && (
        <>
          <div className="tune-table">
            <Head />
            {rows.map((row, index) => {
              const gen = tps(row, "gen");
              const prompt = tps(row, "prompt");
              return (
                <div
                  key={`${row.candidate.ctx}-${row.candidate.cacheK}`}
                  className={`tune-row${row === fastest ? " is-fastest" : ""}`}
                >
                  <span>
                    {describe(row.candidate.ctx, row.candidate.cacheK)}
                    {index === 0 && (
                      <em className="tune-note"> what arithmetic picks</em>
                    )}
                    {row === fastest && <em className="tune-note"> fastest</em>}
                  </span>
                  <span>
                    {row.error
                      ? row.error
                      : gen == null
                        ? "—"
                        : `${gen.toFixed(1)} tok/s`}
                  </span>
                  <span>{prompt == null ? "—" : `${prompt.toFixed(0)} tok/s`}</span>
                </div>
              );
            })}
          </div>

          <Verdict rows={rows} />

          {mine && report.promptWords != null && (
            <p className="field-hint">
              Every candidate answered the same {report.promptWords.toLocaleString()}
              -word prompt, sized to the smallest context so each did identical
              work. Comparing settings on different prompts compares nothing.
            </p>
          )}
          {mine && report.cancelled && (
            <p className="field-hint">Cancelled — the rest were not measured.</p>
          )}
        </>
      )}

      {summary && summary.rows.length > 0 && (
        <>
          <h3 className="tune-history-head">What this model has done</h3>
          <div className="tune-table">
            <Head />
            {summary.rows.map((row) => {
              const suggested =
                summary.suggestion != null &&
                !row.stale &&
                sameKey(row.key, summary.suggestion);
              return (
              <div
                key={`${row.key.ctx}-${row.key.cacheTypeK}-${row.llamaVersion}`}
                className={`tune-row${row.ranked ? "" : " is-unranked"}${suggested ? " is-fastest" : ""}`}
              >
                <span>
                  {keyText(row.key)}
                  {suggested && <em className="tune-note"> suggested</em>}
                  <br />
                  <Workload row={row} />
                </span>
                <span>
                  {row.genTps == null ? "—" : `${row.genTps.toFixed(1)} tok/s`}
                </span>
                <span>
                  {row.promptTps == null
                    ? "—"
                    : `${row.promptTps.toFixed(0)} tok/s`}
                </span>
              </div>
              );
            })}
          </div>
        </>
      )}
    </div>
  );
}
