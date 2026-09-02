import { useEffect, useState, type ReactNode } from "react";
import type { Profile } from "./types";

const CACHE_TYPES = ["f16", "bf16", "q8_0", "q5_1", "q5_0", "q4_1", "q4_0"];
const CTX_FLOOR = 4096;

/// Matches `profile::AUTO_CTX`. A context of 0 means the launch passes no `-c` and
/// llama.cpp sizes one to the memory it can see.
export const AUTO_CTX = 0;

// macOS substitutes a dash for the `--` that starts every llama-server flag, and the
// server rejects what comes out. Only a token's leading dash can be one: substitution
// needs two hyphens, so `--cache-type-k` keeps the single hyphens inside it.
function undoDashSubstitution(text: string) {
  return text.replace(/(^|\s)[–—]/g, "$1--");
}

/// What a context is worth in something a reader can picture. A token is about
/// three-quarters of a word, a page about five hundred words.
function pagesOf(ctx: number): string {
  const pages = Math.round((ctx * 0.75) / 500);
  if (pages < 1) return "Under a page of text in one conversation";
  if (pages === 1) return "About a page of text in one conversation";
  return `Roughly ${pages} pages of text in one conversation`;
}

/// The context choices, largest first: what the file allows, then halves of it down to a
/// floor. Named rather than a slider, because 21 launches changed this field to one of a
/// handful of round numbers and never to anything between them.
function contextChoices(ceiling: number, current: number): number[] {
  const out: number[] = [];
  for (let value = ceiling; value >= CTX_FLOOR; value = Math.floor(value / 2)) {
    out.push(value);
  }
  if (current !== AUTO_CTX && !out.includes(current)) {
    out.push(current);
    out.sort((a, b) => b - a);
  }
  return out;
}

function splitArgs(text: string) {
  return text.split(" ").filter((a) => a.length > 0);
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      {children}
      {hint && <span className="field-hint">{hint}</span>}
    </label>
  );
}

/// The two fields a launch actually changes. Across 21 launches these were the only ones
/// ever touched ([intent/direction.md]), so they are the only ones in the open.
export function ProfileFields({
  value,
  maxCtx,
  fitAvailable,
  onChange,
}: {
  value: Profile;
  maxCtx: number | null;
  fitAvailable: boolean;
  onChange: (next: Profile) => void;
}) {
  const set = <K extends keyof Profile>(key: K, next: Profile[K]) =>
    onChange({ ...value, [key]: next });

  const ceiling = maxCtx ?? 131072;
  const choices = contextChoices(ceiling, value.ctx);

  let ctxHint = pagesOf(value.ctx);
  if (value.ctx === AUTO_CTX) {
    ctxHint = "The server picks the largest that fits, and reports it once running";
  }

  return (
    <div className="fields-row">
      <Field label="How much it can hold" hint={ctxHint}>
        <select
          value={value.ctx}
          onChange={(e) => set("ctx", Number(e.currentTarget.value))}
        >
          {fitAvailable && (
            <option value={AUTO_CTX}>Fitted to memory — recommended</option>
          )}
          {choices.map((choice) => (
            <option key={choice} value={choice}>
              {choice.toLocaleString()} tokens
              {choice === ceiling ? " — the most this model allows" : ""}
            </option>
          ))}
        </select>
      </Field>

      <Field label="Port" hint="The address other apps use to reach it">
        <input
          type="number"
          value={value.port}
          onChange={(e) => set("port", Number(e.currentTarget.value))}
        />
      </Field>
    </div>
  );
}

/// The seven fields nobody has changed. Rendered by whatever wants to fold them away.
export function AdvancedFields({
  value,
  showAlias = true,
  onChange,
}: {
  value: Profile;
  showAlias?: boolean;
  onChange: (next: Profile) => void;
}) {
  const set = <K extends keyof Profile>(key: K, next: Profile[K]) =>
    onChange({ ...value, [key]: next });

  // The field holds text, not the parsed list: joining the list back would delete the
  // space the moment it is typed, and one argument cannot be separated from the next.
  const [rawText, setRawText] = useState(() => value.rawArgs.join(" "));
  const incoming = value.rawArgs.join(" ");
  useEffect(() => {
    if (splitArgs(rawText).join(" ") !== incoming) {
      setRawText(incoming);
    }
  }, [incoming]);

  return (
    <div className="form-grid">
      {showAlias && (
        <Field label="Alias" hint="the name the server reports to clients">
          <input
            value={value.alias}
            onChange={(e) => set("alias", e.currentTarget.value)}
          />
        </Field>
      )}

      <Field
        label="GPU layers"
        hint="auto lets the server place them; all, or a number"
      >
        <input
          value={value.ngl}
          onChange={(e) => set("ngl", e.currentTarget.value)}
        />
      </Field>

      <Field label="Parallel slots">
        <input
          type="number"
          min={1}
          value={value.parallel}
          onChange={(e) => set("parallel", Number(e.currentTarget.value))}
        />
      </Field>

      <Field label="Cache type K">
        <select
          value={value.cacheTypeK}
          onChange={(e) => set("cacheTypeK", e.currentTarget.value)}
        >
          {CACHE_TYPES.map((type) => (
            <option key={type}>{type}</option>
          ))}
        </select>
      </Field>

      <Field label="Cache type V">
        <select
          value={value.cacheTypeV}
          onChange={(e) => set("cacheTypeV", e.currentTarget.value)}
        >
          {CACHE_TYPES.map((type) => (
            <option key={type}>{type}</option>
          ))}
        </select>
      </Field>

      <Field label="Flags">
        <span className="toggles">
          <label className="toggle">
            <input
              type="checkbox"
              checked={value.flashAttn}
              onChange={(e) => set("flashAttn", e.currentTarget.checked)}
            />
            flash attention
          </label>
          <label className="toggle">
            <input
              type="checkbox"
              checked={value.jinja}
              onChange={(e) => set("jinja", e.currentTarget.checked)}
            />
            jinja
          </label>
        </span>
      </Field>

      <Field label="Extra arguments" hint="space separated, passed through verbatim">
        <input
          value={rawText}
          onChange={(e) => {
            const text = undoDashSubstitution(e.currentTarget.value);
            setRawText(text);
            set("rawArgs", splitArgs(text));
          }}
        />
      </Field>
    </div>
  );
}

/// Both halves with the advanced one folded, for screens that want the whole form in one
/// place — Settings' launch defaults.
export default function ProfileForm({
  value,
  maxCtx,
  fitAvailable,
  showAlias = true,
  onChange,
}: {
  value: Profile;
  maxCtx: number | null;
  fitAvailable: boolean;
  showAlias?: boolean;
  onChange: (next: Profile) => void;
}) {
  return (
    <>
      <ProfileFields
        value={value}
        maxCtx={maxCtx}
        fitAvailable={fitAvailable}
        onChange={onChange}
      />
      <details className="advanced">
        <summary>Advanced — the settings you have never had to change</summary>
        <AdvancedFields
          value={value}
          showAlias={showAlias}
          onChange={onChange}
        />
      </details>
    </>
  );
}
