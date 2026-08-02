import { useCallback, useEffect, useState } from "react";
import {
  listModels,
  onCatalogChanged,
  onRunnerLog,
  onRunnerState,
  onTelemetry,
  orphanStatus,
  orphanStop,
  runnerLogs,
  runnerStatus,
  runnerStop,
} from "./api";
import Downloads from "./Downloads";
import Library from "./Library";
import ModelDetail from "./ModelDetail";
import SettingsScreen from "./SettingsScreen";
import type { ModelEntry, Orphan, RunnerSnapshot, Telemetry } from "./types";
import "./App.css";

type Screen = "library" | "downloads" | "settings";

const NAV: { id: Screen; label: string }[] = [
  { id: "library", label: "Library" },
  { id: "downloads", label: "Downloads" },
  { id: "settings", label: "Settings" },
];

const IDLE: RunnerSnapshot = {
  state: "idle",
  modelId: null,
  modelName: null,
  alias: null,
  port: null,
  requestedPort: null,
  pid: null,
  startedSecs: null,
  error: null,
  crashTail: [],
  restarted: false,
  serverCtx: null,
};

function NowRunning({
  runner,
  telemetry,
  onStop,
}: {
  runner: RunnerSnapshot;
  telemetry: Telemetry | null;
  onStop: () => void;
}) {
  const active = runner.state === "starting" || runner.state === "ready";
  const kv = telemetry?.kvCacheUsage;

  return (
    <div className="now-running">
      <span className={`dot state-${runner.state}`} />
      <span className="now-running-text">
        {active ? (
          <>
            <strong>{runner.alias ?? runner.modelName}</strong>
            <span className="now-running-meta">
              {runner.state === "starting"
                ? "starting…"
                : `:${runner.port}${kv != null ? ` · KV ${Math.round(kv * 100)}%` : ""}`}
            </span>
          </>
        ) : (
          "No model running"
        )}
      </span>
      {active && (
        <button className="link-stop" onClick={onStop}>
          Stop
        </button>
      )}
    </div>
  );
}

export default function App() {
  const [screen, setScreen] = useState<Screen>("library");
  const [selected, setSelected] = useState<ModelEntry | null>(null);
  const [runner, setRunner] = useState<RunnerSnapshot>(IDLE);
  const [telemetry, setTelemetry] = useState<Telemetry | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [orphans, setOrphans] = useState<Orphan[]>([]);
  const [catalogVersion, setCatalogVersion] = useState(0);

  useEffect(() => {
    runnerStatus().then(setRunner).catch(() => {});
    runnerLogs().then(setLogs).catch(() => {});
    const scan = () => orphanStatus().then(setOrphans).catch(() => {});
    scan();
    const rescan = setInterval(scan, 10000);

    const unlisten = [
      onRunnerState((snapshot) => {
        setRunner(snapshot);
        if (snapshot.state === "starting") {
          setLogs([]);
          setTelemetry(null);
        }
      }),
      onRunnerLog((line) => setLogs((prev) => [...prev.slice(-1999), line])),
      onTelemetry(setTelemetry),
      onCatalogChanged(() => setCatalogVersion((v) => v + 1)),
    ];

    return () => {
      clearInterval(rescan);
      unlisten.forEach((p) => void p.then((off) => off()));
    };
  }, []);

  const stop = useCallback(() => {
    runnerStop().then(setRunner).catch(() => {});
  }, []);

  const showInLibrary = useCallback((path: string) => {
    setScreen("library");
    listModels()
      .then((models) => {
        const landed = models.find((model) => model.path === path);
        if (landed) setSelected(landed);
      })
      .catch(() => {});
  }, []);

  const content = () => {
    if (selected) {
      return (
        <ModelDetail
          model={selected}
          runner={runner}
          telemetry={telemetry}
          logs={logs}
          onBack={() => setSelected(null)}
          onRunnerChange={setRunner}
        />
      );
    }
    if (screen === "downloads") {
      return <Downloads onShowInLibrary={showInLibrary} />;
    }
    if (screen === "settings") {
      return (
        <SettingsScreen
          onModelsDirChanged={() => setCatalogVersion((v) => v + 1)}
        />
      );
    }
    return (
      <Library key={catalogVersion} runner={runner} onSelect={setSelected} />
    );
  };

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="sidebar-title">llama.cpp hub</div>
        <ul className="nav">
          {NAV.map((item) => (
            <li key={item.id}>
              <button
                className={`nav-item${item.id === screen && !selected ? " is-active" : ""}`}
                onClick={() => {
                  setSelected(null);
                  setScreen(item.id);
                }}
              >
                {item.label}
              </button>
            </li>
          ))}
        </ul>
        <NowRunning runner={runner} telemetry={telemetry} onStop={stop} />
      </nav>

      <main className="content">
        {orphans.length > 0 && (
          <div className="notice">
            {orphans.length === 1
              ? "A llama-server is running that this window did not start:"
              : `${orphans.length} llama-servers are running that this window did not start:`}
            <ul className="orphan-list">
              {orphans.map((orphan) => (
                <li key={orphan.pid}>
                  <span>
                    {orphan.model ?? "unknown model"}
                    {orphan.port != null && ` · port ${orphan.port}`} · pid{" "}
                    {orphan.pid}
                  </span>
                  <button
                    className="button"
                    onClick={() =>
                      orphanStop(orphan.pid)
                        .then(setOrphans)
                        .catch(() => {})
                    }
                  >
                    Stop
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}
        {content()}
      </main>
    </div>
  );
}
