import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  getSettings,
  setAppearance,
  setLaunchDefaults,
  setLlamaServerPath,
  setModelsDir,
} from "./api";
import Disclosure from "./Disclosure";
import { formatContext } from "./format";
import ProfileForm from "./ProfileForm";
import {
  apply as applyAppearance,
  modeOf,
  MODES,
  THEMES,
  themeOf,
  type Mode,
} from "./theme";
import { AUTO_CTX } from "./ProfileForm";
import type { Capabilities, Profile, Settings } from "./types";

/// What this app asks of the build it finds. `--fit` is not here: without it the app
/// resolves Auto to a number itself, which is a smaller loss than a missing figure.
const NEEDED = ["--metrics", "--cache-type-k"];

/// One line about the binary, in place of the six facts this card used to print. It says
/// which flag is missing rather than only that something is: "not supported" without a
/// name is a sentence nobody can act on.
function binaryVerdict(
  settings: Settings,
  caps: Capabilities,
  missing: string[],
): string {
  let found = "Found automatically";
  if (settings.llamaServerPath != null) {
    found = "Chosen by you";
  }
  const version = caps.version ?? "an unnamed build";

  if (missing.length === 0) {
    return `${found} · version ${version} · everything Llamaport needs is supported`;
  }
  return `${found} · version ${version} · this build has no ${missing.join(" and no ")}`;
}

/// What a model opens on before anyone has launched it. Mirrors `Profile::default()` in
/// Rust; the two only have to agree on what the form shows, because a launch sends the
/// whole profile rather than the fields that differ from it.
export default function SettingsScreen({
  onModelsDirChanged,
}: {
  onModelsDirChanged: () => void;
}) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [dir, setDir] = useState("");
  const [saved, setSaved] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [defaults, setDefaults] = useState<Profile | null>(null);

  useEffect(() => {
    getSettings()
      .then((next) => {
        setSettings(next);
        setDir(next.modelsDir);
        setDefaults(next.launchDefaults);
      })
      .catch((e) => setFailure(String(e)));
  }, []);

  const flash = (message: string) => {
    setSaved(message);
    setTimeout(() => setSaved(null), 2000);
  };

  if (!settings) {
    return (
      <>
        <header className="screen-header">
          <h1>Settings</h1>
        </header>
        {failure && <p className="notice notice-error">{failure}</p>}
      </>
    );
  }

  const caps = settings.capabilities;
  const missing = NEEDED.filter((flag) => !(caps?.flags ?? []).includes(flag));

  // The one line the fold leaves behind, so what the defaults are does not need opening.
  const shown = defaults ?? settings.builtInDefaults;
  let source = "Built-in";
  if (settings.launchDefaults != null) {
    source = "Yours";
  }
  let context = "fitted context";
  if (shown.ctx !== AUTO_CTX) {
    context = `${formatContext(shown.ctx)} context`;
  }
  const defaultsSummary = `${source} · ${context} · port ${shown.port}`;

  const theme = themeOf(settings.appearance);
  const mode = modeOf(settings.appearance);

  // Applied from what Rust wrote back rather than from what was clicked: the screen then
  // shows the palette that is actually stored, even if the write failed.
  const saveAppearance = (nextTheme: string, nextMode: Mode) => {
    setAppearance({ theme: nextTheme, mode: nextMode })
      .then((next) => {
        setSettings(next);
        applyAppearance(next.appearance);
      })
      .catch((e) => setFailure(String(e)));
  };

  return (
    <>
      <header className="screen-header">
        <h1>Settings</h1>
        {saved && <span className="field-hint">{saved}</span>}
      </header>

      {failure && <p className="notice notice-error">{failure}</p>}

      <section className="panel">
        <h2>Models folder</h2>
        <p className="field-hint">
          Where your models live on disk. Nothing watches it, so press Rescan in
          the Library after adding a file by hand.
        </p>
        <div className="row-input">
          <input value={dir} readOnly />
          <button
            className="button"
            onClick={() => {
              setFailure(null);
              open({ directory: true, defaultPath: dir })
                .then((chosen) => {
                  if (typeof chosen !== "string") return;
                  setDir(chosen);
                  return setModelsDir(chosen).then(() => {
                    onModelsDirChanged();
                    flash("saved");
                  });
                })
                .catch((e) => setFailure(String(e)));
            }}
          >
            Change…
          </button>
        </div>
      </section>

      <section className="panel">
        <h2>llama-server</h2>

        {settings.capabilityError && (
          <p className="notice notice-error">{settings.capabilityError}</p>
        )}

        {caps && (
          <p className="verdict-line">
            <span className={`dot tone-${missing.length === 0 ? "ok" : "warn"}`} />
            <span>{binaryVerdict(settings, caps, missing)}</span>
          </p>
        )}

        <p className="field-hint path-line">
          <span title={caps?.binary}>{caps?.binary ?? "not found"}</span>
          <button
            className="button button-link"
            onClick={() => {
              setFailure(null);
              open({ directory: false, defaultPath: caps?.binary })
                .then((chosen) => {
                  if (typeof chosen !== "string") return;
                  return setLlamaServerPath(chosen).then((next) => {
                    setSettings(next);
                    flash("saved");
                  });
                })
                .catch((e) => setFailure(String(e)));
            }}
          >
            Choose a different one
          </button>
          {settings.llamaServerPath != null && (
            <button
              className="button button-link"
              onClick={() =>
                setLlamaServerPath(null)
                  .then((next) => {
                    setSettings(next);
                    flash("searching again");
                  })
                  .catch((e) => setFailure(String(e)))
              }
            >
              Find it for me
            </button>
          )}
        </p>
      </section>

      <section className="panel">
        <h2>Appearance</h2>
        <p className="field-hint">
          Mode belongs to the Llamaport palette, which has a light and a dark
          set. Every other palette is one appearance and says which.
        </p>

        <div className="mode-row">
          {MODES.map((option) => (
            <button
              key={option.id}
              className={`button${mode === option.id ? " button-primary" : ""}`}
              onClick={() => saveAppearance(theme.id, option.id)}
            >
              {option.label}
            </button>
          ))}
          {theme.fixed && (
            <span className="field-hint">
              {theme.label} is a {theme.fixed} theme — mode applies to Llamaport
            </span>
          )}
        </div>

        <div className="themes">
          {THEMES.map((option) => (
            <button
              key={option.id}
              className={`theme-row${option.id === theme.id ? " is-selected" : ""}`}
              onClick={() => saveAppearance(option.id, mode)}
            >
              <span className="swatch">
                {option.swatch.map((colour) => (
                  <span key={colour} style={{ background: colour }} />
                ))}
              </span>
              <span>
                <span className="theme-name">{option.label}</span>
                <span className="field-hint">{option.desc}</span>
              </span>
            </button>
          ))}
        </div>
      </section>

      <section className="panel">
        <h2>Launch defaults</h2>
        <p className="field-hint">
          Where a model opens the first time you launch it. A model you have
          already launched opens on its own last successful launch instead, so
          changing these never overwrites anything you have tuned.
        </p>

        <Disclosure title="Edit defaults" sub={defaultsSummary}>
          <ProfileForm
            fitAvailable={caps?.flags.includes("--fit") ?? false}
            value={defaults ?? settings.builtInDefaults}
            maxCtx={null}
            showAlias={false}
            onChange={setDefaults}
          />

          <div className="panel-actions">
            <button
              className="button button-primary"
              onClick={() =>
                setLaunchDefaults(defaults)
                  .then((next) => {
                    setSettings(next);
                    setDefaults(next.launchDefaults);
                    flash("saved");
                  })
                  .catch((e) => setFailure(String(e)))
              }
            >
              Save defaults
            </button>
            {settings.launchDefaults && (
              <button
                className="button"
                onClick={() =>
                  setLaunchDefaults(null)
                    .then((next) => {
                      setSettings(next);
                      setDefaults(next.launchDefaults);
                      flash("back to the built-in values");
                    })
                    .catch((e) => setFailure(String(e)))
                }
              >
                Reset
              </button>
            )}
          </div>
        </Disclosure>
      </section>
    </>
  );
}
