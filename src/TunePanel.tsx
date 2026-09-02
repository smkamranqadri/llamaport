import Disclosure from "./Disclosure";
import { CheckIcon } from "./icons";
import type {
  Profile,
  SpeedKey,
  SpeedRow,
  SpeedSummary,
  TuneCandidate,
  TuneOutcome,
  TuneReport,
} from "./types";

const CACHE_WORDS: Record<string, string> = {
  f16: "full precision",
  bf16: "full precision",
  q8_0: "compact memory",
  q5_1: "smaller memory",
  q5_0: "smaller memory",
  q4_1: "smallest memory",
  q4_0: "smallest memory",
};

/// The ladder's rungs are powers of two, and every one of them reads as a round number of
/// thousands to the person choosing between them.
function contextWords(ctx: number) {
  if (ctx >= 1024 && ctx % 1024 === 0) {
    return `${(ctx / 1024).toLocaleString()}k context`;
  }
  return `${ctx.toLocaleString()} context`;
}

function candidateName(candidate: TuneCandidate) {
  return `${contextWords(candidate.ctx)} · ${CACHE_WORDS[candidate.cacheK] ?? candidate.cacheK}`;
}

/// The figures the name is a translation of. They stay beside it: a person comparing two
/// tries reads the words, and a person checking what was launched needs the arguments.
function candidateExact(candidate: TuneCandidate) {
  return `${candidate.ctx.toLocaleString()} · ${candidate.cacheK}`;
}

function keyText(key: SpeedKey) {
  return `${key.ctx.toLocaleString()} · ${key.cacheTypeK}`;
}

function tps(outcome: TuneOutcome, of: "gen" | "prompt"): number | null {
  const reading = outcome.reading;
  if (reading == null) return null;
  const tokens = of === "gen" ? reading.genTokens : reading.promptTokens;
  const seconds = of === "gen" ? reading.genSeconds : reading.promptSeconds;
  if (seconds <= 0) return null;
  return tokens / seconds;
}

function sameCandidate(a: TuneCandidate, b: TuneCandidate) {
  return a.ctx === b.ctx && a.cacheK === b.cacheK && a.cacheV === b.cacheV;
}

function sameKey(a: SpeedKey, b: SpeedKey) {
  return a.ctx === b.ctx && a.cacheTypeK === b.cacheTypeK && a.cacheTypeV === b.cacheTypeV;
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

function History({ summary }: { summary: SpeedSummary }) {
  return (
    <div className="tune-table">
      <div className="tune-row tune-head">
        <span>Context · cache</span>
        <span>Generation</span>
        <span>Prompt eval</span>
      </div>
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
              {row.promptTps == null ? "—" : `${row.promptTps.toFixed(0)} tok/s`}
            </span>
          </div>
        );
      })}
    </div>
  );
}

/// One try: what it is in words, the arguments behind them, and what it got. A try that
/// has not run yet is drawn too — a list that grows from nothing never says how long it
/// has left to go.
function Try({
  candidate,
  outcome,
  state,
  note,
  marked,
  markedWord,
}: {
  candidate: TuneCandidate;
  outcome: TuneOutcome | null;
  state: "done" | "measuring" | "waiting";
  note: string | null;
  /// The row the sentence and the button below the list are about.
  marked: boolean;
  markedWord: string;
}) {
  const gen = outcome == null ? null : tps(outcome, "gen");

  let result = <span className="tune-try-result">—</span>;
  if (state === "measuring") {
    result = (
      <span className="bar tune-progress">
        <span />
      </span>
    );
  } else if (outcome?.error != null) {
    result = <span className="tune-try-result">did not load</span>;
  } else if (gen != null) {
    let text = `${gen.toFixed(0)} tok/s`;
    if (marked) {
      text = `${text} · ${markedWord}`;
    }
    result = <span className="tune-try-result">{text}</span>;
  }

  const parts = [candidateExact(candidate)];
  if (note) parts.push(note);

  return (
    <div className={`tune-try${marked ? " is-fastest" : ""}`}>
      <span className="tune-try-name">{candidateName(candidate)}</span>
      <span className="tune-try-note">{parts.join(" — ")}</span>
      {result}
    </div>
  );
}

/// The ladder as it runs: every combination it will try, what each one got, and the one
/// sentence that says what to do about it. Everything the reader would only want if they
/// disagreed with that sentence is folded away underneath.
export default function TunePanel({
  report,
  summary,
  onApply,
}: {
  /// The ladder's own report, always for the model this screen shows.
  report: TuneReport;
  summary: SpeedSummary | null;
  onApply: (settings: Partial<Profile>) => void;
}) {
  const running = report.running;
  const rows = report.rows;

  // The report names its rungs up front, but a report already in flight from an older
  // build does not, and neither does one restored before the field existed.
  let ladder = report.candidates;
  if (ladder.length === 0) {
    ladder = rows.map((row) => row.candidate);
  }

  const fastest = rows
    .filter((row) => tps(row, "gen") != null)
    .reduce<TuneOutcome | null>(
      (best, row) => (best == null || tps(row, "gen")! > tps(best, "gen")! ? row : best),
      null,
    );

  // Once it is over the marked row is the one the button applies, which is not always the
  // quickest reading: the suggestion prefers the widest context among readings too close
  // to separate. A green row naming one setting over a sentence naming another is the
  // screen disagreeing with itself.
  let marked = fastest;
  let markedWord = "fastest so far";
  if (!running) {
    markedWord = "fastest";
    const key = summary?.suggestion;
    if (key != null) {
      const suggested = rows.find(
        (row) =>
          row.candidate.ctx === key.ctx &&
          row.candidate.cacheK === key.cacheTypeK &&
          row.candidate.cacheV === key.cacheTypeV,
      );
      if (suggested != null) {
        marked = suggested;
        markedWord = "suggested";
      }
    }
  }

  const apply = (key: SpeedKey) =>
    onApply({
      ctx: key.ctx,
      cacheTypeK: key.cacheTypeK,
      cacheTypeV: key.cacheTypeV,
      ngl: key.ngl,
      flashAttn: key.flashAttn,
      parallel: key.parallel,
    });

  // While the ladder runs, the only thing that can be applied is the leader; once it is
  // over, the suggestion is the answer, because it carries the tie rule the leader does
  // not — the widest context among readings too close to separate.
  let action: (() => void) | null = null;
  let actionLabel = "Use fastest so far";
  if (running && fastest != null) {
    const candidate = fastest.candidate;
    action = () =>
      onApply({ ctx: candidate.ctx, cacheTypeK: candidate.cacheK, cacheTypeV: candidate.cacheV });
  } else if (!running && summary?.suggestion != null) {
    const key = summary.suggestion;
    actionLabel = "Use these settings";
    action = () => apply(key);
  }

  let foot = "When it finishes, the fastest becomes your Best speed preset for this model.";
  if (!running) {
    foot = "The fastest is now your Best speed preset for this model.";
  }
  if (report.cancelled) {
    foot = "Cancelled — the rest were not measured. The fastest of those that ran is your Best speed preset.";
  }

  let verdict: string | null = null;
  if (!running && summary?.suggestion != null && summary.suggestedTps != null) {
    verdict = `${summary.suggestedTps.toFixed(0)} tok/s at ${candidateName({
      ctx: summary.suggestion.ctx,
      cacheK: summary.suggestion.cacheTypeK,
      cacheV: summary.suggestion.cacheTypeV,
    })}`;
    if (summary.beats != null && summary.beatsByPercent != null) {
      verdict += ` — ${Math.round(summary.beatsByPercent)}% faster than ${keyText(summary.beats)}, which is what fitting the largest context would have chosen.`;
    } else {
      verdict += " — the largest that fits was also the fastest, so nothing was traded.";
    }
  }

  const caveats = summary != null && (summary.tied > 1 || summary.confidence === "observed");
  const why = caveats || report.promptWords != null;

  let intro =
    "Llamaport launched this model at each of these settings and asked every one of them the same question.";
  if (running) {
    intro =
      "Llamaport tries a few safe combinations of context size and memory precision, and times each one. The model restarts between tries — this takes a couple of minutes.";
  }

  let whySub = "what the reading is worth";
  if (summary != null && summary.tied > 1) {
    whySub = `${summary.tied} settings within 10% of each other`;
  }

  return (
    <div className="tune">
      <p className="tune-intro">{intro}</p>

      <p className="group-label">
        Tries — {report.done} of {report.total} done
      </p>

      <div className="tune-tries">
        {ladder.map((candidate, index) => {
          const outcome = rows.find((row) => sameCandidate(row.candidate, candidate)) ?? null;
          let state: "done" | "measuring" | "waiting" = "waiting";
          if (outcome != null) {
            state = "done";
          } else if (report.current != null && sameCandidate(report.current, candidate)) {
            state = "measuring";
          }

          let note: string | null = null;
          if (state === "measuring") {
            note = "measuring…";
          } else if (state === "waiting") {
            note = running ? "waiting" : "not measured";
          } else if (outcome?.error != null) {
            note = outcome.error;
          } else if (index === 0) {
            note = "the largest that fits";
          } else if (candidate.cacheK !== "f16" && candidate.cacheK !== "bf16") {
            note = "half the memory, slight quality trade";
          }

          return (
            <Try
              key={`${candidate.ctx}-${candidate.cacheK}-${candidate.cacheV}`}
              candidate={candidate}
              outcome={outcome}
              state={state}
              note={note}
              marked={outcome != null && outcome === marked}
              markedWord={markedWord}
            />
          );
        })}
      </div>

      <div className="tune-foot">
        <span className="field-hint">{foot}</span>
        <button className="button button-primary" disabled={action == null} onClick={() => action?.()}>
          <CheckIcon />
          {actionLabel}
        </button>
      </div>

      {verdict && <p className="tune-verdict">{verdict}</p>}

      {why && summary != null && (
        <Disclosure
          flat
          title="Why this one"
          sub={whySub}
        >
          {summary.tied > 1 && (
            <p className="field-hint">
              {summary.tied} settings measured within 10% of each other, which is
              inside what repeated runs of the same pair disagree by. The widest
              context among them is suggested rather than the fastest reading —
              context costs nothing when the speed is the same.
            </p>
          )}
          {summary.confidence === "observed" && (
            <p className="field-hint">
              From ordinary use, not a measurement: those runs answered different
              questions, so the ordering can be wrong. Measuring asks every
              setting the same one.
            </p>
          )}
          {report.promptWords != null && (
            <p className="field-hint">
              Every candidate answered the same{" "}
              {report.promptWords.toLocaleString()}-word prompt, sized to the
              smallest context so each did identical work. Comparing settings on
              different prompts compares nothing.
            </p>
          )}
        </Disclosure>
      )}

      {summary != null && summary.rows.length > 0 && (
        <Disclosure
          flat
          title="What this model has done"
          sub={`${summary.rows.length} settings recorded`}
        >
          <History summary={summary} />
        </Disclosure>
      )}
    </div>
  );
}
