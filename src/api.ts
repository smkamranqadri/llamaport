import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  NamedProfile,
  DirInfo,
  HealthReport,
  LaunchPlan,
  ModelEntry,
  Profile,
  ProfilePatch,
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
  draft?: ProfilePatch,
): Promise<LaunchPlan> {
  return invoke<LaunchPlan>("launch_plan", { modelId, draft: draft ?? null });
}

export function saveProfile(
  modelId: string,
  patch: ProfilePatch,
): Promise<LaunchPlan> {
  return invoke<LaunchPlan>("save_profile", { modelId, patch });
}

export function saveDefaultProfile(profile: Profile): Promise<void> {
  return invoke<void>("save_default_profile", { profile });
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
  draft?: ProfilePatch,
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

export function listProfiles(): Promise<NamedProfile[]> {
  return invoke<NamedProfile[]>("list_profiles");
}

export function orphanStatus(): Promise<Orphan[]> {
  return invoke<Orphan[]>("orphan_status");
}

export function orphanStop(pid: number): Promise<Orphan[]> {
  return invoke<Orphan[]>("orphan_stop", { pid });
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
