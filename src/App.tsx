import { useCallback, useEffect, useState } from "react";
import {
  onRunnerLog,
  onRunnerState,
  onTelemetry,
  orphanDismiss,
  orphanStatus,
  orphanStop,
  runnerLogs,
  runnerStatus,
  runnerStop,
} from "./api";
import Benchmarks from "./Benchmarks";
import Library from "./Library";
import ModelDetail from "./ModelDetail";
import SettingsScreen from "./SettingsScreen";
import type { ModelEntry, Orphan, RunnerSnapshot, Telemetry } from "./types";
import "./App.css";

type Screen = "library" | "benchmarks" | "discover" | "downloads" | "settings";

const NAV: { id: Screen; label: string; phase?: string }[] = [
  { id: "library", label: "Library" },
  { id: "benchmarks", label: "Benchmarks" },
  { id: "discover", label: "Discover", phase: "4" },
  { id: "downloads", label: "Downloads", phase: "3" },
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

function Placeholder({ label, phase }: { label: string; phase: string }) {
  return (
    <>
      <header className="screen-header">
        <h1>{label}</h1>
      </header>
      <div className="empty">
        <p className="empty-title">Not built yet</p>
        <p className="empty-detail">Arrives in phase {phase}.</p>
      </div>
    </>
  );
}

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
  const [orphan, setOrphan] = useState<Orphan | null>(null);
  const [catalogVersion, setCatalogVersion] = useState(0);

  useEffect(() => {
    runnerStatus().then(setRunner).catch(() => {});
    runnerLogs().then(setLogs).catch(() => {});
    orphanStatus().then(setOrphan).catch(() => {});

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
    ];

    return () => {
      unlisten.forEach((p) => void p.then((off) => off()));
    };
  }, []);

  const stop = useCallback(() => {
    runnerStop().then(setRunner).catch(() => {});
  }, []);

  const active = NAV.find((item) => item.id === screen)!;

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
    if (screen === "library") {
      return (
        <Library
          key={catalogVersion}
          runner={runner}
          onSelect={setSelected}
        />
      );
    }
    if (screen === "benchmarks") {
      return <Benchmarks />;
    }
    if (screen === "settings") {
      return (
        <SettingsScreen
          onModelsDirChanged={() => setCatalogVersion((v) => v + 1)}
        />
      );
    }
    return <Placeholder label={active.label} phase={active.phase!} />;
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
        {orphan && (
          <div className="notice">
            An llama-server from a previous session is still running on port{" "}
            {orphan.port} (pid {orphan.pid}). It was not started by this window.
            <span className="notice-actions">
              <button
                className="button"
                onClick={() =>
                  orphanStop(orphan.pid)
                    .then(() => setOrphan(null))
                    .catch(() => setOrphan(null))
                }
              >
                Stop it
              </button>
              <button
                className="button"
                onClick={() => orphanDismiss().then(() => setOrphan(null))}
              >
                Leave it running
              </button>
            </span>
          </div>
        )}
        {content()}
      </main>
    </div>
  );
}
