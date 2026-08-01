import type { NamedProfile, Profile, ProfilePatch } from "./types";

/// Alias, host and port stay out: they belong to a model and an instance, not to a
/// workload. A template that stamped them would break the model it was applied to.
const WORKLOAD_FIELDS = [
  "ctx",
  "ngl",
  "parallel",
  "flashAttn",
  "cacheTypeK",
  "cacheTypeV",
  "jinja",
  "rawArgs",
] as const satisfies readonly (keyof Profile)[];

export function workloadPatch(form: Profile): ProfilePatch {
  const patch: Record<string, unknown> = {};
  for (const field of WORKLOAD_FIELDS) {
    patch[field] = form[field];
  }
  return patch as ProfilePatch;
}

export function applyPatch(form: Profile, patch: ProfilePatch): Profile {
  const set = Object.entries(patch).filter(([, value]) => value != null);
  return { ...form, ...Object.fromEntries(set) } as Profile;
}

export function summarise(patch: ProfilePatch): string {
  const parts: string[] = [];
  if (patch.ctx != null) parts.push(`${(patch.ctx / 1024).toFixed(0)}K context`);
  if (patch.cacheTypeK != null && patch.cacheTypeV != null) {
    parts.push(
      patch.cacheTypeK === patch.cacheTypeV
        ? `${patch.cacheTypeK} cache`
        : `cache ${patch.cacheTypeK}/${patch.cacheTypeV}`,
    );
  }
  if (patch.parallel != null) parts.push(`${patch.parallel} slot${patch.parallel === 1 ? "" : "s"}`);
  if (patch.ngl != null) parts.push(`ngl ${patch.ngl}`);
  if (patch.flashAttn === false) parts.push("no flash attention");
  if (patch.jinja === false) parts.push("no jinja");
  if (patch.rawArgs?.length) parts.push(`${patch.rawArgs.length} extra arg(s)`);
  return parts.join(" · ") || "no settings";
}

export function ProfileBadge({ profile }: { profile: NamedProfile }) {
  return (
    <span className={`badge ${profile.builtIn ? "badge-quiet" : "badge-moe"}`}>
      {profile.builtIn ? "built-in" : "yours"}
    </span>
  );
}
