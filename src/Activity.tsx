import { useEffect, useRef, useState } from "react";
import { activitySnapshot } from "./api";
import { formatMemory } from "./format";
import { pressureText } from "./Memory";
import { Card } from "./Telemetry";
import type { Activity, ActivityProcess } from "./types";

/// How often the machine is asked. A CPU percentage is a difference between two polls, so
/// this is also the window every figure in the table is averaged over.
const EVERY_MS = 2000;

const SPARK_POINTS = 40;

/// What a row is, said rather than colour-coded: a stray server is the one thing on this
/// screen the app did not start, and the whole point of showing it is that it is not ours.
function what(row: ActivityProcess): string {
  if (row.kind === "measurement") return "measuring best speed";
  if (row.kind === "stray") return "left over — this app did not start it";
  return "running";
}

function tone(row: ActivityProcess): "ok" | "warn" {
  if (row.kind === "stray") return "warn";
  return "ok";
}

export default function ActivityScreen() {
  const [activity, setActivity] = useState<Activity | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const history = useRef<number[]>([]);
  const [spark, setSpark] = useState<number[]>([]);

  useEffect(() => {
    let live = true;
    const read = () => {
      activitySnapshot()
        .then((next) => {
          if (!live) return;
          setActivity(next);
          setFailure(null);
          if (next.totalCpuPercent != null) {
            history.current = [...history.current, next.totalCpuPercent].slice(
              -SPARK_POINTS,
            );
            setSpark(history.current);
          }
        })
        .catch((e) => {
          if (live) setFailure(String(e));
        });
    };

    read();
    const timer = setInterval(read, EVERY_MS);
    return () => {
      live = false;
      clearInterval(timer);
    };
  }, []);

  const rows = activity?.processes ?? [];

  let cpuValue = "—";
  if (activity?.totalCpuPercent != null) {
    cpuValue = `${activity.totalCpuPercent.toFixed(1)}%`;
  }

  let memoryValue = "—";
  let memorySub = "what the whole Mac is using";
  let memoryFill: number | null = null;
  if (activity?.memoryUsedBytes != null && activity.memoryTotalBytes != null) {
    memoryValue = formatMemory(activity.memoryUsedBytes);
    memorySub = `of ${formatMemory(activity.memoryTotalBytes)}`;
    memoryFill = activity.memoryUsedBytes / activity.memoryTotalBytes;
  }

  // The GPU cannot say what it is doing — macOS publishes no per-process figure and no
  // overall one — but it can say what it will hand out, which is what a launch has to fit
  // inside. That is the figure this card carries instead of a utilisation percentage.
  let gpuValue = "Unknown";
  let gpuSub = "this build of llama-server does not report the GPU's limit";
  let gpuFill: number | null = null;
  const budget = activity?.deviceBudgetBytes ?? null;
  const wanted = activity?.gpuWantedBytes ?? null;
  if (budget != null) {
    gpuValue = formatMemory(budget);
    gpuSub = "the most the GPU will hand out to a launch";
    if (wanted != null) {
      gpuValue = formatMemory(wanted);
      gpuSub = `of ${formatMemory(budget)} the GPU will hand out`;
      gpuFill = wanted / budget;
    }
  }

  let swapValue = "—";
  if (activity?.swapUsedBytes != null) {
    swapValue = formatMemory(activity.swapUsedBytes);
  }
  let swapSub = "nothing is being paged out";
  let swapTone: "ok" | "bad" | undefined;
  if (activity != null && activity.pressure !== "unknown") {
    swapSub = `memory pressure is ${pressureText(activity.pressure)}`;
    swapTone = "ok";
    if (activity.pressure !== "normal") swapTone = "bad";
  }

  return (
    <>
      <header className="screen-header">
        <div>
          <h1>Activity Monitor</h1>
          <p className="screen-subtitle">What your models cost right now</p>
        </div>
      </header>

      {failure && <p className="notice notice-error">{failure}</p>}

      <div className="activity-table">
        <div className="activity-row activity-head">
          <span>Name</span>
          <span>Memory</span>
          <span>CPU</span>
        </div>
        {rows.map((row) => (
          <div key={row.pid} className="activity-row">
            <span className="activity-name">
              <span className={`dot tone-${tone(row)}`} />
              <span className="activity-title">{row.name}</span>
              <span className="activity-what">
                {what(row)}
                {row.port != null && ` · port ${row.port}`}
              </span>
            </span>
            <span>
              {row.memoryBytes == null ? "—" : formatMemory(row.memoryBytes)}
            </span>
            <span>
              {row.cpuPercent == null ? "—" : `${row.cpuPercent.toFixed(1)}%`}
            </span>
          </div>
        ))}
        {rows.length === 0 && (
          <p className="empty-detail activity-empty">
            No model is running, and no llama-server of any kind is on this Mac.
            The figures below are the machine's own.
          </p>
        )}
      </div>

      <div className="cards activity-cards">
        <Card
          label="Total CPU"
          value={cpuValue}
          sub="across the whole Mac"
          spark={spark}
          capacity={SPARK_POINTS}
        />
        <Card label="Memory" value={memoryValue} sub={memorySub} fill={memoryFill} />
        <Card label="GPU memory" value={gpuValue} sub={gpuSub} fill={gpuFill} />
        <Card
          label="Swap"
          value={swapValue}
          sub={swapSub}
          subTone={swapTone}
        />
      </div>
    </>
  );
}
