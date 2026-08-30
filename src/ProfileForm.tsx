import { useEffect, useState, type ReactNode } from "react";
import type { Profile } from "./types";

const CACHE_TYPES = ["f16", "bf16", "q8_0", "q5_1", "q5_0", "q4_1", "q4_0"];
const CTX_MIN = 4096;
const CTX_STEP = 1024;

// macOS substitutes a dash for the `--` that starts every llama-server flag, and the
// server rejects what comes out. Only a token's leading dash can be one: substitution
// needs two hyphens, so `--cache-type-k` keeps the single hyphens inside it.
function undoDashSubstitution(text: string) {
  return text.replace(/(^|\s)[–—]/g, "$1--");
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

export default function ProfileForm({
  value,
  maxCtx,
  showAlias = true,
  onChange,
}: {
  value: Profile;
  maxCtx: number | null;
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

  const ctxCeiling = maxCtx ?? 131072;

  return (
    <div className="form-grid">
      {showAlias && (
        <Field
          label="Alias"
        >
          <input
            value={value.alias}
            onChange={(e) => set("alias", e.currentTarget.value)}
          />
        </Field>
      )}

      <Field
        label="Port"
      >
        <input
          type="number"
          value={value.port}
          onChange={(e) => set("port", Number(e.currentTarget.value))}
        />
      </Field>

      <Field
        label={`Context — ${value.ctx.toLocaleString()} tokens`}
        hint={maxCtx ? `model maximum ${maxCtx.toLocaleString()}` : undefined}
      >
        <>
          <input
            type="range"
            min={CTX_MIN}
            max={ctxCeiling}
            step={CTX_STEP}
            value={Math.min(value.ctx, ctxCeiling)}
            onChange={(e) => set("ctx", Number(e.currentTarget.value))}
          />
        </>
      </Field>

      <Field
        label="GPU layers"
        hint="all, auto, or a number"
      >
        <input
          value={value.ngl}
          onChange={(e) => set("ngl", e.currentTarget.value)}
        />
      </Field>

      <Field
        label="Parallel slots"
      >
        <input
          type="number"
          min={1}
          value={value.parallel}
          onChange={(e) => set("parallel", Number(e.currentTarget.value))}
        />
      </Field>

      <Field
        label="Cache type K"
      >
        <select
          value={value.cacheTypeK}
          onChange={(e) => set("cacheTypeK", e.currentTarget.value)}
        >
          {CACHE_TYPES.map((type) => (
            <option key={type}>{type}</option>
          ))}
        </select>
      </Field>

      <Field
        label="Cache type V"
      >
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

      <Field
        label="Extra arguments"
        hint="space separated, passed through verbatim"
      >
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
