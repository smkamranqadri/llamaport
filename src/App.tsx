import { useCallback, useEffect, useState } from "react";
import {
  appVersion,
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
import {
  ChartIcon,
  CompassIcon,
  DownloadIcon,
  SlidersIcon,
  StackIcon,
  StopIcon,
} from "./icons";
import Library from "./Library";
import ModelDetail from "./ModelDetail";
import SettingsScreen from "./SettingsScreen";
import type { ModelEntry, Orphan, RunnerSnapshot, Telemetry } from "./types";
import "./App.css";

type Screen = "library" | "downloads" | "settings";

function NavItem({
  label,
  icon,
  active,
  disabled,
  extra,
  onClick,
}: {
  label: string;
  icon: React.ReactNode;
  active?: boolean;
  disabled?: boolean;
  extra?: React.ReactNode;
  onClick?: () => void;
}) {
  return (
    <button
      className={`nav-item${active ? " is-active" : ""}`}
      disabled={disabled}
      title={disabled ? "Coming soon" : undefined}
      onClick={onClick}
    >
      {icon}
      <span className="nav-label">{label}</span>
      {extra}
    </button>
  );
}

const IDLE: RunnerSnapshot = {
  state: "idle",
  modelId: null,
  modelName: null,
  alias: null,
  port: null,
  pid: null,
  startedSecs: null,
  error: null,
  crashTail: [],
  restarted: false,
  serverCtx: null,
};

/// One line per stray server, as the artboard draws it: what it is on the left, Stop it
/// and Ignore on the right. It used to stack a bullet list under a two-line heading.
function OrphanBanner({
  orphans,
  onStopped,
  onIgnore,
}: {
  orphans: Orphan[];
  onStopped: (remaining: Orphan[]) => void;
  onIgnore: () => void;
}) {
  return (
    <>
      {orphans.map((orphan) => {
        const facts = [orphan.model ?? "unknown model"];
        if (orphan.port != null) facts.push(`port ${orphan.port}`);
        facts.push("probably left over from a previous session or started from the terminal");

        return (
          <div className="banner" key={orphan.pid}>
            <div className="banner-body">
              <span className="banner-title">
                A llama-server is running that Llamaport did not start
              </span>
              <span className="banner-detail">{facts.join(" · ")}</span>
            </div>
            <button
              className="button"
              onClick={() =>
                orphanStop(orphan.pid).then(onStopped).catch(() => {})
              }
            >
              <StopIcon />
              Stop it
            </button>
            <button className="button button-plain" onClick={onIgnore}>
              Ignore
            </button>
          </div>
        );
      })}
    </>
  );
}

export default function App() {
  const [screen, setScreen] = useState<Screen>("library");
  const [selected, setSelected] = useState<ModelEntry | null>(null);
  const [runner, setRunner] = useState<RunnerSnapshot>(IDLE);
  const [telemetry, setTelemetry] = useState<Telemetry | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [orphans, setOrphans] = useState<Orphan[]>([]);
  const [ignoredOrphans, setIgnoredOrphans] = useState<number[]>([]);
  const [catalogVersion, setCatalogVersion] = useState(0);
  const [version, setVersion] = useState("");

  useEffect(() => {
    appVersion().then(setVersion).catch(() => {});
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

  const go = (next: Screen) => {
    setSelected(null);
    setScreen(next);
  };

  const running = runner.state === "starting" || runner.state === "ready";
  const visibleOrphans = orphans.filter(
    (orphan) => !ignoredOrphans.includes(orphan.pid),
  );

  const content = () => {
    if (selected) {
      return (
        <ModelDetail
          model={selected}
          runner={runner}
          telemetry={telemetry}
          logs={logs}
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
      <Library
        key={catalogVersion}
        runner={runner}
        telemetry={telemetry}
        onSelect={setSelected}
        onStop={stop}
        onRunnerChange={setRunner}
      />
    );
  };

  return (
    <div className="app">
      <nav className="sidebar">
        <div className="side-section">Models</div>
        <NavItem
          label="Library"
          icon={<StackIcon />}
          active={screen === "library" && !selected}
          extra={running ? <span className="side-run-dot" /> : undefined}
          onClick={() => go("library")}
        />
        <NavItem label="Discover" icon={<CompassIcon />} disabled />
        <NavItem
          label="Downloads"
          icon={<DownloadIcon />}
          active={screen === "downloads" && !selected}
          onClick={() => go("downloads")}
        />
        <div className="side-section">General</div>
        <NavItem label="Activity Monitor" icon={<ChartIcon />} disabled />
        <div className="side-spacer" />
        <NavItem
          label="Settings"
          icon={<SlidersIcon />}
          active={screen === "settings" && !selected}
          onClick={() => go("settings")}
        />
        <div className="side-version">Llamaport {version}</div>
      </nav>

      <main className="content">
        {visibleOrphans.length > 0 && (
          <OrphanBanner
            orphans={visibleOrphans}
            onStopped={setOrphans}
            onIgnore={() =>
              setIgnoredOrphans(orphans.map((orphan) => orphan.pid))
            }
          />
        )}
        {content()}
      </main>
    </div>
  );
}
