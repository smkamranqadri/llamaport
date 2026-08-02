import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  DirInfo,
  DownloadJob,
  DownloadProgress,
  HealthReport,
  LaunchPlan,
  ModelEntry,
  Profile,
  RunnerSnapshot,
  Orphan,
  Settings,
  Telemetry,
} from "./types";

export function listModels(): Promise<ModelEntry[]> {
  return invoke<ModelEntry[]>("catalog_list");
}

export function getDirInfo(): Promise<DirInfo> {
  return invoke<DirInfo>("catalog_dir_info");
}

export function setModelsDir(path: string): Promise<DirInfo> {
  return invoke<DirInfo>("set_models_dir", { path });
}

export function getLaunchPlan(
  modelId: string,
  draft?: Profile,
): Promise<LaunchPlan> {
  return invoke<LaunchPlan>("launch_plan", { modelId, draft: draft ?? null });
}

export function setLlamaServerPath(path: string | null): Promise<Settings> {
  return invoke<Settings>("set_llama_server_path", { path });
}

export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export function runnerStatus(): Promise<RunnerSnapshot> {
  return invoke<RunnerSnapshot>("runner_status");
}

export function runnerLogs(): Promise<string[]> {
  return invoke<string[]>("runner_logs");
}

export function runnerStart(
  modelId: string,
  draft?: Profile,
): Promise<RunnerSnapshot> {
  return invoke<RunnerSnapshot>("runner_start", {
    modelId,
    draft: draft ?? null,
  });
}

export function runnerStop(): Promise<RunnerSnapshot> {
  return invoke<RunnerSnapshot>("runner_stop");
}

export function revealPath(path: string): Promise<void> {
  return invoke<void>("reveal_path", { path });
}

export function healthTest(): Promise<HealthReport> {
  return invoke<HealthReport>("health_test");
}

export function orphanStatus(): Promise<Orphan[]> {
  return invoke<Orphan[]>("orphan_status");
}

export function orphanStop(pid: number): Promise<Orphan[]> {
  return invoke<Orphan[]>("orphan_stop", { pid });
}

export function downloadStart(url: string): Promise<DownloadJob[]> {
  return invoke<DownloadJob[]>("download_start", { url });
}

export function downloadCancel(id: string): Promise<DownloadJob[]> {
  return invoke<DownloadJob[]>("download_cancel", { id });
}

export function downloadStatus(): Promise<DownloadJob[]> {
  return invoke<DownloadJob[]>("download_status");
}

export function downloadClear(): Promise<DownloadJob[]> {
  return invoke<DownloadJob[]>("download_clear");
}

export function onRunnerState(handler: (snapshot: RunnerSnapshot) => void) {
  return listen<RunnerSnapshot>("runner:state", (event) =>
    handler(event.payload),
  );
}

export function onRunnerLog(handler: (line: string) => void) {
  return listen<string>("runner:log", (event) => handler(event.payload));
}

export function onTelemetry(handler: (telemetry: Telemetry) => void) {
  return listen<Telemetry>("runner:telemetry", (event) =>
    handler(event.payload),
  );
}

export function onDownloadState(handler: (jobs: DownloadJob[]) => void) {
  return listen<DownloadJob[]>("download:state", (event) =>
    handler(event.payload),
  );
}

export function onDownloadProgress(
  handler: (progress: DownloadProgress) => void,
) {
  return listen<DownloadProgress>("download:progress", (event) =>
    handler(event.payload),
  );
}

export function onCatalogChanged(handler: (models: ModelEntry[]) => void) {
  return listen<ModelEntry[]>("catalog:changed", (event) =>
    handler(event.payload),
  );
}
