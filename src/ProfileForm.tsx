import type { ReactNode } from "react";
import type { Profile } from "./types";

const CACHE_TYPES = ["f16", "bf16", "q8_0", "q5_1", "q5_0", "q4_1", "q4_0"];
const CTX_STEP = 4096;

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
  practicalCtx,
  riskyCtx,
  showAlias = true,
  onChange,
}: {
  value: Profile;
  maxCtx: number | null;
  practicalCtx?: number | null;
  riskyCtx?: number | null;
  showAlias?: boolean;
  onChange: (next: Profile) => void;
}) {
  const set = <K extends keyof Profile>(key: K, next: Profile[K]) =>
    onChange({ ...value, [key]: next });

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
