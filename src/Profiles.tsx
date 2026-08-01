import type { Profile, ProfilePatch } from "./types";

export function applyPatch(form: Profile, patch: ProfilePatch): Profile {
  const set = Object.entries(patch).filter(([, value]) => value != null);
  return { ...form, ...Object.fromEntries(set) } as Profile;
}
