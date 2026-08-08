export interface GgufMetadata {
  ggufVersion: number;
  tensorCount: number;
  architecture: string;
  name: string | null;
  sizeLabel: string | null;
  contextLength: number | null;
  blockCount: number | null;
  embeddingLength: number | null;
  headCount: number | null;
  headCountKv: number | null;
  keyLength: number | null;
  valueLength: number | null;
  expertCount: number | null;
  fileType: number | null;
  hasChatTemplate: boolean;
}

export interface ShardInfo {
  total: number;
  present: number;
  missing: number[];
}

export interface ModelEntry {
  id: string;
  displayName: string;
  fileName: string;
  path: string;
  sizeBytes: number;
  modifiedSecs: number | null;
  quant: string | null;
  shards: ShardInfo | null;
  metadata: GgufMetadata | null;
  error: string | null;
  favourite: boolean;
  /// Null until the model has been launched to Ready at least once.
  lastLaunchedSecs: number | null;
}

export interface DirInfo {
  path: string;
  exists: boolean;
  freeBytes: number | null;
  totalBytes: number | null;
}

export interface Profile {
  alias: string;
  host: string;
  port: number;
  ctx: number;
  ngl: string;
  parallel: number;
  flashAttn: boolean;
  cacheTypeK: string;
  cacheTypeV: string;
  jinja: boolean;
  rawArgs: string[];
}

export interface Estimate {
  weightsBytes: number;
  kvBytes: number;
  totalBytes: number;
}

export interface LaunchPlan {
  profile: Profile;
  args: string[];
  command: string;
  estimate: Estimate | null;
  totalMemory: number;
  memory: PlanMemory;
  maxCtx: number | null;
  portConflict: {
    port: number;
    respondsToHealth: boolean;
    isLlamaServer: boolean;
  } | null;
  capabilityError: string | null;
}

export type RunState = "idle" | "starting" | "ready" | "stopping" | "crashed";

export interface RunnerSnapshot {
  state: RunState;
  modelId: string | null;
  modelName: string | null;
  alias: string | null;
  port: number | null;
  pid: number | null;
  startedSecs: number | null;
  error: string | null;
  crashTail: string[];
  restarted: boolean;
  serverCtx: number | null;
}

export type Pressure = "normal" | "warning" | "critical" | "unknown";

export interface PlanMemory {
  installedBytes: number | null;
  usedBytes: number | null;
  swapUsedBytes: number | null;
  pressure: Pressure;
}

export interface Orphan {
  pid: number;
  port: number | null;
  model: string | null;
}

export interface Telemetry {
  kvCacheUsage: number | null;
  promptTps: number | null;
  genTps: number | null;
  lastPromptTps: number | null;
  lastGenTps: number | null;
  tokensGenerated: number | null;
  tokensPrompt: number | null;
  requestsProcessing: number | null;
  requestsDeferred: number | null;
  systemUsedBytes: number | null;
  systemTotalBytes: number | null;
  swapUsedBytes: number | null;
  processFootprintBytes: number | null;
  pressure: Pressure;
  healthOk: boolean;
  uptimeSecs: number;
}

export type DownloadPhase = "resolving" | "transferring" | "verifying";
export type DownloadState =
  | "active"
  | "queued"
  | "paused"
  | "complete"
  | "failed";

export interface DownloadJob {
  id: string;
  url: string;
  fileName: string;
  path: string;
  state: DownloadState;
  phase: DownloadPhase | null;
  completed: number;
  total: number | null;
  bytesPerSecond: number | null;
  error: string | null;
  startedSecs: number | null;
  finishedSecs: number | null;
  /// False only on a paused transfer whose bytes no longer back its sidecar.
  resumable: boolean;
}

export interface DownloadProgress {
  id: string;
  phase: DownloadPhase;
  completed: number;
  total: number | null;
  bytesPerSecond: number | null;
}

export type CheckStatus = "passed" | "warning" | "failed" | "skipped";
export type Verdict = "passed" | "passedWithWarnings" | "failed";
export type Reasoning = "separateField" | "inline" | "notReturned";

export interface HealthCheck {
  name: string;
  status: CheckStatus;
  detail: string;
  durationMs: number;
}

export interface HealthTimings {
  timeToFirstTokenMs: number | null;
  totalResponseMs: number | null;
  promptTokens: number | null;
  generatedTokens: number | null;
  promptTps: number | null;
  genTps: number | null;
}

export interface HealthReport {
  verdict: Verdict;
  checks: HealthCheck[];
  timings: HealthTimings;
  reasoning: Reasoning;
}

export interface Capabilities {
  binary: string;
  version: string | null;
  flags: string[];
  flashAttnTakesValue: boolean;
}

export interface DownloadOptions {
  segments: number;
  /// Bytes per second across the whole transfer. Null is unlimited.
  rateLimit: number | null;
  verify: boolean;
}

export interface Settings {
  modelsDir: string;
  llamaServerPath: string | null;
  downloads: DownloadOptions;
  /// Null until the user sets them: the built-in values are then in force.
  launchDefaults: Profile | null;
  capabilities: Capabilities | null;
  capabilityError: string | null;
}
