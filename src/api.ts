import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AgentConnectionInfo,
  BenchmarkQuery,
  BenchmarkRecord,
  DirInfo,
  HealthReport,
  LaunchPlan,
  PiInspection,
  PiPreview,
  ModelEntry,
  NamedProfile,
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

export function healthTest(): Promise<HealthReport> {
  return invoke<HealthReport>("health_test");
}

export function benchmarkRun(
  depthTokens: number,
  generateTokens: number,
): Promise<BenchmarkRecord> {
  return invoke<BenchmarkRecord>("benchmark_run", {
    depthTokens,
    generateTokens,
  });
}

export function agentConnection(): Promise<AgentConnectionInfo> {
  return invoke<AgentConnectionInfo>("agent_connection");
}

export function piInspect(): Promise<PiInspection> {
  return invoke<PiInspection>("pi_inspect");
}

export function piPreview(providerName?: string): Promise<PiPreview> {
  return invoke<PiPreview>("pi_preview", {
    providerName: providerName ?? null,
  });
}

export function benchmarksList(
  query?: BenchmarkQuery,
): Promise<BenchmarkRecord[]> {
  return invoke<BenchmarkRecord[]>("benchmarks_list", { query: query ?? null });
}

export function benchmarkDelete(id: string): Promise<BenchmarkRecord[]> {
  return invoke<BenchmarkRecord[]>("benchmark_delete", { id });
}

export function benchmarkNote(
  id: string,
  note: string | null,
): Promise<BenchmarkRecord[]> {
  return invoke<BenchmarkRecord[]>("benchmark_note", { id, note });
}

export function benchmarksExport(format: "csv" | "json"): Promise<string> {
  return invoke<string>("benchmarks_export", { format });
}

export function listProfiles(): Promise<NamedProfile[]> {
  return invoke<NamedProfile[]>("list_profiles");
}

export function saveNamedProfile(profile: NamedProfile): Promise<NamedProfile[]> {
  return invoke<NamedProfile[]>("save_named_profile", { profile });
}

export function duplicateProfile(id: string): Promise<NamedProfile[]> {
  return invoke<NamedProfile[]>("duplicate_profile", { id });
}

export function deleteProfile(id: string): Promise<NamedProfile[]> {
  return invoke<NamedProfile[]>("delete_profile", { id });
}

export function resetProfile(id: string): Promise<NamedProfile[]> {
  return invoke<NamedProfile[]>("reset_profile", { id });
}

export function orphanStatus(): Promise<Orphan | null> {
  return invoke<Orphan | null>("orphan_status");
}

export function orphanStop(pid: number): Promise<void> {
  return invoke<void>("orphan_stop", { pid });
}

export function orphanDismiss(): Promise<void> {
  return invoke<void>("orphan_dismiss");
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
