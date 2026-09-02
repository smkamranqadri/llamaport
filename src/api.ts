import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  DirInfo,
  DownloadJob,
  DownloadOptions,
  DownloadProgress,
  HealthReport,
  LaunchPlan,
  MachineMemory,
  ModelEntry,
  Profile,
  RunnerSnapshot,
  Orphan,
  PiPreview,
  Settings,
  SpeedSummary,
  Telemetry,
  TuneReport,
} from "./types";

export function listModels(): Promise<ModelEntry[]> {
  return invoke<ModelEntry[]>("catalog_list");
}

export function appVersion(): Promise<string> {
  return invoke<string>("app_version");
}

export function setFavourite(
  modelId: string,
  favourite: boolean,
): Promise<ModelEntry[]> {
  return invoke<ModelEntry[]>("model_favourite", { modelId, favourite });
}

export function deleteModel(modelId: string): Promise<ModelEntry[]> {
  return invoke<ModelEntry[]>("model_delete", { modelId });
}

export function machineMemory(): Promise<MachineMemory> {
  return invoke<MachineMemory>("machine_memory");
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

export function setDownloadOptions(
  options: DownloadOptions,
): Promise<Settings> {
  return invoke<Settings>("set_download_options", { options });
}

export function setLaunchDefaults(
  defaults: Profile | null,
): Promise<Settings> {
  return invoke<Settings>("set_launch_defaults", { defaults });
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

export function openWebUi(port: number): Promise<void> {
  return invoke<void>("open_webui_window", { port });
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

export function speedsFor(modelId: string): Promise<SpeedSummary> {
  return invoke<SpeedSummary>("speeds_for", { modelId });
}

export function tuneStatus(): Promise<TuneReport> {
  return invoke<TuneReport>("tune_status");
}

export function tuneStart(modelId: string): Promise<TuneReport> {
  return invoke<TuneReport>("tune_start", { modelId });
}

export function tuneCancel(): Promise<TuneReport> {
  return invoke<TuneReport>("tune_cancel");
}

export function downloadStart(url: string): Promise<DownloadJob[]> {
  return invoke<DownloadJob[]>("download_start", { url });
}

export function downloadPause(id: string): Promise<DownloadJob[]> {
  return invoke<DownloadJob[]>("download_pause", { id });
}

export function downloadResume(id: string): Promise<DownloadJob[]> {
  return invoke<DownloadJob[]>("download_resume", { id });
}

export function downloadDiscard(id: string): Promise<DownloadJob[]> {
  return invoke<DownloadJob[]>("download_discard", { id });
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

export function piPreview(): Promise<PiPreview> {
  return invoke<PiPreview>("pi_preview");
}

export function piApply(reasoning: boolean): Promise<PiPreview> {
  return invoke<PiPreview>("pi_apply", { reasoning });
}

export function onTelemetry(handler: (telemetry: Telemetry) => void) {
  return listen<Telemetry>("runner:telemetry", (event) =>
    handler(event.payload),
  );
}

export function onTuneReport(handler: (report: TuneReport) => void) {
  return listen<TuneReport>("tune:report", (event) => handler(event.payload));
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
