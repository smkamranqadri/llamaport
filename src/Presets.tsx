import { AUTO_CTX } from "./ProfileForm";
import type { Profile, SpeedKey, SpeedSummary } from "./types";

/// The fields a preset owns. Alias, port, jinja and extra arguments are the user's and
/// no preset touches them.
const SPEED_KEYS = [
  "ctx",
  "ngl",
  "parallel",
  "cacheTypeK",
  "cacheTypeV",
  "flashAttn",
] as const;

type SpeedFields = Pick<Profile, (typeof SPEED_KEYS)[number]>;

function matches(form: Profile, partial: Partial<SpeedFields>): boolean {
  return Object.entries(partial).every(
    ([key, wanted]) => form[key as keyof SpeedFields] === wanted,
  );
}

function pick(profile: Profile): SpeedFields {
  return {
    ctx: profile.ctx,
    ngl: profile.ngl,
    parallel: profile.parallel,
    cacheTypeK: profile.cacheTypeK,
    cacheTypeV: profile.cacheTypeV,
    flashAttn: profile.flashAttn,
  };
}

// A div rather than a button: the disabled Best-speed card carries a live Measure
// button, and a button may not contain one.
function Card({
  title,
  detail,
  hint,
  selected,
  disabled,
  action,
  onPick,
}: {
  title: string;
  detail: string;
  hint?: string;
  selected: boolean;
  disabled?: boolean;
  action?: React.ReactNode;
  onPick: () => void;
}) {
  const pick = disabled ? undefined : onPick;
  return (
    <div
      className={`preset${selected ? " is-selected" : ""}${disabled ? " is-disabled" : ""}`}
      role="button"
      aria-disabled={disabled}
      tabIndex={disabled ? -1 : 0}
      onClick={pick}
      onKeyDown={(e) => {
        if (pick && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          pick();
        }
      }}
    >
      <span className="preset-head">
        <span className={`radio${selected ? " is-on" : ""}`} />
        <span className="preset-title">{title}</span>
      </span>
      <span className="preset-detail">{detail}</span>
      {(hint || action) && (
        <span className="preset-foot">
          {hint && <span className="field-hint">{hint}</span>}
          {hint && action && <span className="field-hint">·</span>}
          {action}
        </span>
      )}
    </div>
  );
}

export type Which = "best" | "fitted" | "default" | null;

/// Which named way to run the form currently holds, or null for settings picked by hand.
/// One answer even where values coincide: a measured pick outranks the fit, and the fit
/// outranks defaults that happen to equal it. The screen's other copy reads this too, so
/// nothing on it can name a different preset than the highlighted card.
export function selectedPreset({
  form,
  defaults,
  summary,
  fitAvailable,
}: {
  form: Profile;
  defaults: Profile | null;
  summary: SpeedSummary | null;
  fitAvailable: boolean;
}): Which {
  const suggestion = summary?.suggestion ?? null;
  if (suggestion && matches(form, suggestionFields(suggestion))) return "best";
  if (fitAvailable && matches(form, { ctx: AUTO_CTX, ngl: "auto" })) {
    return "fitted";
  }
  if (defaults && matches(form, pick(defaults))) return "default";
  return null;
}

export function presetName(which: Which): string | null {
  if (which === "best") return "Best speed";
  if (which === "fitted") return "Model suggested";
  if (which === "default") return "Default";
  return null;
}

function suggestionFields(key: SpeedKey): SpeedFields {
  return {
    ctx: key.ctx,
    ngl: key.ngl,
    parallel: key.parallel,
    cacheTypeK: key.cacheTypeK,
    cacheTypeV: key.cacheTypeV,
    flashAttn: key.flashAttn,
  };
}

/// Three named ways to run, over machinery that already exists: the launch defaults, the
/// measured suggestion in speeds.json, and the server fitting what the header allows.
/// A preset whose backing fact is missing renders disabled with the action that would
/// create it.
export default function Presets({
  form,
  defaults,
  summary,
  fitAvailable,
  measureBlocked,
  measuring,
  picked,
  onApply,
  onMeasure,
}: {
  form: Profile;
  defaults: Profile | null;
  summary: SpeedSummary | null;
  fitAvailable: boolean;
  measureBlocked: string | null;
  measuring: boolean;
  /// The card the user last pressed, which wins over the derived answer while the form
  /// still holds what it applied. Two presets can carry identical settings — the built-in
  /// defaults are the fit — and deriving alone then lights the wrong card.
  picked: Which;
  onApply: (partial: Partial<Profile>, which: Which) => void;
  onMeasure: () => void;
}) {
  const suggestion = summary?.suggestion ?? null;
  const measured = summary?.confidence === "tuned";

  const bestPartial = suggestion == null ? null : suggestionFields(suggestion);
  const fittedPartial: Partial<SpeedFields> = { ctx: AUTO_CTX, ngl: "auto" };
  const defaultPartial = defaults == null ? null : pick(defaults);

  const derived = selectedPreset({ form, defaults, summary, fitAvailable });
  const selected = picked ?? derived;

  // Two cards can hold the same settings — the built-in defaults are "fit to memory" and
  // "auto layers", which is what the server would choose anyway. Saying so is better than
  // leaving the reader to wonder why one click lights a different card.
  const defaultIsFit =
    defaultPartial != null &&
    fitAvailable &&
    defaultPartial.ctx === AUTO_CTX &&
    defaultPartial.ngl === "auto";

  // What the card says about a measurement it already has: when it was taken, or that it
  // was not taken at all and the figure is only what ordinary use happened to show.
  let bestHint: string | undefined;
  if (suggestion != null) {
    const row = summary?.rows.find(
      (r) =>
        r.key.ctx === suggestion.ctx &&
        r.key.cacheTypeK === suggestion.cacheTypeK &&
        !r.stale,
    );
    if (measured && row) {
      bestHint = `Measured here on ${new Date(row.timestampSecs * 1000).toLocaleDateString(undefined, { month: "short", day: "numeric" })}`;
    } else if (measured) {
      bestHint = "Measured on this Mac";
    } else {
      bestHint = "From ordinary use, not a measurement";
    }
  }

  const measureButton = (
    <button
      className="button button-link"
      disabled={measureBlocked != null || measuring}
      title={measureBlocked ?? undefined}
      onClick={(e) => {
        e.stopPropagation();
        onMeasure();
      }}
    >
      {measuring ? "Measuring…" : suggestion == null ? "Measure" : "Measure again"}
    </button>
  );

  return (
    <>
      <div className="presets">
        <Card
          title="Default"
          detail="Safe settings that work on any Mac."
          hint={
            defaultIsFit
              ? "The same as Model suggested, until you save your own in Settings"
              : undefined
          }
          selected={selected === "default"}
          disabled={defaults == null}
          onPick={() => defaultPartial && onApply(defaultPartial, "default")}
        />
        <Card
          title="Best speed"
          detail="The fastest settings measured on this Mac."
          hint={suggestion == null ? "Not measured yet" : bestHint}
          selected={selected === "best"}
          disabled={suggestion == null}
          action={measureButton}
          onPick={() => bestPartial && onApply(bestPartial, "best")}
        />
        <Card
          title="Model suggested"
          detail="What the model file itself recommends."
          hint={
            fitAvailable
              ? undefined
              : "This llama-server build cannot fit settings on its own"
          }
          selected={selected === "fitted"}
          disabled={!fitAvailable}
          onPick={() => onApply(fittedPartial, "fitted")}
        />
      </div>
      {selected == null && (
        <p className="field-hint">
          Custom settings — picked by hand, kept from this model's last launch.
        </p>
      )}
    </>
  );
}
