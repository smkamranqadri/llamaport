import type { ReactNode } from "react";
import type { Profile } from "./types";

const CACHE_TYPES = ["f16", "bf16", "q8_0", "q5_1", "q5_0", "q4_1", "q4_0"];
const CTX_STEP = 4096;

export function diffProfile(form: Profile, defaults: Profile): Partial<Profile> {
  const patch: Record<string, unknown> = {};
  for (const key of Object.keys(form) as (keyof Profile)[]) {
    const mine = form[key];
    const theirs = defaults[key];
    const same = Array.isArray(mine)
      ? JSON.stringify(mine) === JSON.stringify(theirs)
      : mine === theirs;
    if (!same) patch[key] = mine;
  }
  return patch as Partial<Profile>;
}

function Field({
  label,
  hint,
  overridden,
  onReset,
  children,
}: {
  label: string;
  hint?: string;
  overridden?: boolean;
  onReset?: () => void;
  children: ReactNode;
}) {
  return (
    <label className="field">
      <span className="field-label">
        {label}
        {overridden && (
          <button className="field-reset" onClick={onReset} type="button">
            reset
          </button>
        )}
      </span>
      {children}
      {hint && <span className="field-hint">{hint}</span>}
    </label>
  );
}

export default function ProfileForm({
  value,
  defaults,
  maxCtx,
  practicalCtx,
  riskyCtx,
  showAlias = true,
  onChange,
}: {
  value: Profile;
  defaults: Profile;
  maxCtx: number | null;
  practicalCtx?: number | null;
  riskyCtx?: number | null;
  showAlias?: boolean;
  onChange: (next: Profile) => void;
}) {
  const set = <K extends keyof Profile>(key: K, next: Profile[K]) =>
    onChange({ ...value, [key]: next });

  const reset = <K extends keyof Profile>(key: K) => set(key, defaults[key]);
  const isOverridden = <K extends keyof Profile>(key: K) =>
    JSON.stringify(value[key]) !== JSON.stringify(defaults[key]);

  const ctxCeiling = maxCtx ?? 131072;

  return (
    <div className="form-grid">
      {showAlias && (
        <Field
          label="Alias"
          overridden={isOverridden("alias")}
          onReset={() => reset("alias")}
        >
          <input
            value={value.alias}
            onChange={(e) => set("alias", e.currentTarget.value)}
          />
        </Field>
      )}

      <Field
        label="Port"
        overridden={isOverridden("port")}
        onReset={() => reset("port")}
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
        overridden={isOverridden("ctx")}
        onReset={() => reset("ctx")}
      >
        <>
          <input
            type="range"
            min={CTX_STEP}
            max={ctxCeiling}
            step={CTX_STEP}
            value={Math.min(value.ctx, ctxCeiling)}
            onChange={(e) => set("ctx", Number(e.currentTarget.value))}
          />
          {(practicalCtx != null || riskyCtx != null) && (
            <span className="ctx-scale">
              {practicalCtx != null && (
                <span
                  className="ctx-mark ctx-mark-comfortable"
                  style={{ left: `${Math.min(100, (practicalCtx / ctxCeiling) * 100)}%` }}
                  title={`Comfortable up to about ${practicalCtx.toLocaleString()} tokens on this machine right now`}
                />
              )}
              {riskyCtx != null && (
                <span
                  className="ctx-mark ctx-mark-risky"
                  style={{ left: `${Math.min(100, (riskyCtx / ctxCeiling) * 100)}%` }}
                  title={`Beyond about ${riskyCtx.toLocaleString()} tokens the prediction turns red`}
                />
              )}
            </span>
          )}
        </>
      </Field>

      <Field
        label="GPU layers"
        hint="all, auto, or a number"
        overridden={isOverridden("ngl")}
        onReset={() => reset("ngl")}
      >
        <input
          value={value.ngl}
          onChange={(e) => set("ngl", e.currentTarget.value)}
        />
      </Field>

      <Field
        label="Parallel slots"
        overridden={isOverridden("parallel")}
        onReset={() => reset("parallel")}
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
        overridden={isOverridden("cacheTypeK")}
        onReset={() => reset("cacheTypeK")}
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
        overridden={isOverridden("cacheTypeV")}
        onReset={() => reset("cacheTypeV")}
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
        overridden={isOverridden("rawArgs")}
        onReset={() => reset("rawArgs")}
      >
        <input
          value={value.rawArgs.join(" ")}
          onChange={(e) =>
            set(
              "rawArgs",
              e.currentTarget.value.split(" ").filter((a) => a.length > 0),
            )
          }
        />
      </Field>
    </div>
  );
}
