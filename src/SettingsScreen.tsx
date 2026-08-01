import { useEffect, useState } from "react";
import { getSettings, setLlamaServerPath, setModelsDir } from "./api";
import type { Settings } from "./types";

export default function SettingsScreen({
  onModelsDirChanged,
}: {
  onModelsDirChanged: () => void;
}) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [dir, setDir] = useState("");
  const [binary, setBinary] = useState("");
  const [saved, setSaved] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  useEffect(() => {
    getSettings()
      .then((next) => {
        setSettings(next);
        setDir(next.modelsDir);
        setBinary(next.llamaServerPath ?? "");
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

  return (
    <>
      <header className="screen-header">
        <h1>Settings</h1>
        {saved && <span className="field-hint">{saved}</span>}
      </header>

      {failure && <p className="notice notice-error">{failure}</p>}

      <section className="panel">
        <h2>Models directory</h2>
        <div className="row-input">
          <input value={dir} onChange={(e) => setDir(e.currentTarget.value)} />
          <button
            className="button"
            onClick={() =>
              setModelsDir(dir)
                .then(() => {
                  onModelsDirChanged();
                  flash("saved");
                })
                .catch((e) => setFailure(String(e)))
            }
          >
            Save
          </button>
        </div>
      </section>

      <section className="panel">
        <h2>llama-server</h2>
        <div className="row-input">
          <input
            value={binary}
            placeholder="leave empty to search PATH and the usual locations"
            onChange={(e) => setBinary(e.currentTarget.value)}
          />
          <button
            className="button"
            onClick={() =>
              setLlamaServerPath(binary.trim() === "" ? null : binary.trim())
                .then((next) => {
                  setSettings(next);
                  flash("saved");
                })
                .catch((e) => setFailure(String(e)))
            }
          >
            Save
          </button>
        </div>

        {settings.capabilityError && (
          <p className="notice notice-error">{settings.capabilityError}</p>
        )}

        {caps && (
          <dl className="facts">
            <div className="fact">
              <dt>Binary</dt>
              <dd>{caps.binary}</dd>
            </div>
            <div className="fact">
              <dt>Version</dt>
              <dd>{caps.version ?? "unknown"}</dd>
            </div>
            <div className="fact">
              <dt>Flags detected</dt>
              <dd>{caps.flags.length}</dd>
            </div>
            <div className="fact">
              <dt>--flash-attn</dt>
              <dd>{caps.flashAttnTakesValue ? "takes on/off" : "bare switch"}</dd>
            </div>
            <div className="fact">
              <dt>--metrics</dt>
              <dd>{caps.flags.includes("--metrics") ? "supported" : "missing"}</dd>
            </div>
            <div className="fact">
              <dt>Cache type flags</dt>
              <dd>
                {caps.flags.includes("--cache-type-k") ? "supported" : "missing"}
              </dd>
            </div>
          </dl>
        )}
      </section>

      <section className="panel">
        <h2>Memory calibration</h2>
        <p className="field-hint">
          {settings.calibrationSamples === 0
            ? "No runs recorded yet. Until three runs are observed the estimate uses the nominal figure, which over-predicts on Apple Silicon."
            : `${settings.calibrationSamples} run${settings.calibrationSamples === 1 ? "" : "s"} recorded.`}
          {settings.fittedResidency != null &&
            ` Fitted residency: ${Math.round(settings.fittedResidency * 100)}% of nominal weights plus KV cache.`}
        </p>
      </section>

    </>
  );
}
