import type { Appearance } from "./types";

export type Mode = "system" | "light" | "dark";

export interface Theme {
  id: string;
  label: string;
  desc: string;
  /// Three colours off the palette, drawn as chips beside its name.
  swatch: [string, string, string];
  /// A ported palette is one appearance and nothing else; only the built-in one has both,
  /// which is why Mode applies to it alone.
  fixed: "light" | "dark" | null;
}

/// The built-in palette plus four from hermes-hq, whose ids, names and descriptions
/// are that project's own (`frontend/src/theme.ts`). Their values live in `App.css`.
export const THEMES: Theme[] = [
  {
    id: "llamaport",
    label: "Llamaport",
    desc: "The app's own — light and dark, following the mode above",
    swatch: ["#1e1e20", "#4a9bff", "#32d74b"],
    fixed: null,
  },
  {
    id: "violet",
    label: "Violet",
    desc: "Mission Control — violet and cyan on deep ink",
    swatch: ["#15151f", "#8b5cf6", "#7dd3fc"],
    fixed: "dark",
  },
  {
    id: "nous",
    label: "Nous",
    desc: "Dark teal, cream text, amber accent",
    swatch: ["#041c1c", "#ffe6cb", "#ffac02"],
    fixed: "dark",
  },
  {
    id: "bronze",
    label: "Bronze",
    desc: "Charcoal with bronze accents",
    swatch: ["#0d0f12", "#b98a44", "#4c88c7"],
    fixed: "dark",
  },
  {
    id: "slate",
    label: "Slate",
    desc: "Cool grey-blue, GitHub-like",
    swatch: ["#0d1117", "#7eb8f6", "#63d0a6"],
    fixed: "dark",
  },
];

export const MODES: { id: Mode; label: string }[] = [
  { id: "system", label: "System" },
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
];

export const DEFAULT_APPEARANCE: Appearance = {
  theme: "llamaport",
  mode: "system",
  translucent: false,
};

/// A name this build does not know falls back rather than blanking the window — the
/// config it came from is untrusted input, and a later build may have written it.
export function themeOf(appearance: Appearance | null): Theme {
  const found = THEMES.find((theme) => theme.id === appearance?.theme);
  return found ?? THEMES[0];
}

export function modeOf(appearance: Appearance | null): Mode {
  const mode = appearance?.mode;
  if (mode === "light" || mode === "dark" || mode === "system") return mode;
  return "system";
}

function systemIsDark() {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

/// Light or dark, with nothing left to resolve: a fixed palette answers for itself, and
/// the built-in one asks macOS only when the mode is System.
export function resolve(theme: Theme, mode: Mode): "light" | "dark" {
  if (theme.fixed) return theme.fixed;
  if (mode === "light" || mode === "dark") return mode;
  if (systemIsDark()) return "dark";
  return "light";
}

/// A copy in the webview so the window opens in the right colours. The config is the
/// truth and this is a cache of it: without one the first paint is light whatever the
/// user chose, because reading the config is a round trip to Rust.
const KEY = "llamaport-appearance";

function cache(appearance: Appearance) {
  try {
    localStorage.setItem(KEY, JSON.stringify(appearance));
  } catch {
    // A webview with storage denied still themes itself; it just forgets between launches.
  }
}

function cached(): Appearance | null {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw == null) return null;
    const parsed = JSON.parse(raw) as Appearance;
    if (typeof parsed?.theme !== "string" || typeof parsed?.mode !== "string") return null;
    return parsed;
  } catch {
    return null;
  }
}

export function apply(appearance: Appearance | null) {
  const theme = themeOf(appearance);
  const mode = modeOf(appearance);
  const root = document.documentElement;
  root.setAttribute("data-theme", theme.id);
  root.setAttribute("data-mode", resolve(theme, mode));
  // An attribute rather than a class, like the other two, so one stylesheet decides what
  // translucency means per palette instead of the components knowing.
  root.toggleAttribute("data-translucent", appearance?.translucent === true);
  cache({ theme: theme.id, mode, translucent: appearance?.translucent === true });
}

/// Applied before the first render, from the cache, so the window never opens in the
/// wrong colours and correct itself a moment later.
export function initAppearance() {
  apply(cached() ?? DEFAULT_APPEARANCE);
}

/// macOS switching appearance under a window set to System. Returns the unsubscribe.
export function watchSystem(onChange: () => void) {
  const query = window.matchMedia("(prefers-color-scheme: dark)");
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
}
