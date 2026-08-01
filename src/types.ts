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

export type ProfilePatch = Partial<Profile>;

export interface Estimate {
  weightsBytes: number;
  kvBytes: number;
  overheadBytes: number;
  totalBytes: number;
  calibrated: boolean;
}

export interface LaunchPlan {
  profile: Profile;
  effectiveDefaults: Profile;
  overridden: string[];
  args: string[];
  command: string;
  estimate: Estimate | null;
  totalMemory: number;
  maxCtx: number | null;
  capabilityError: string | null;
}

export type RunState = "idle" | "starting" | "ready" | "stopping" | "crashed";

export interface RunnerSnapshot {
  state: RunState;
  modelId: string | null;
  modelName: string | null;
  alias: string | null;
  port: number | null;
  requestedPort: number | null;
  pid: number | null;
  startedSecs: number | null;
  error: string | null;
  crashTail: string[];
  restarted: boolean;
  serverCtx: number | null;
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
  modelDeltaBytes: number | null;
  uptimeSecs: number;
}

export interface Capabilities {
  binary: string;
  version: string | null;
  flags: string[];
  flashAttnTakesValue: boolean;
}

export interface Settings {
  modelsDir: string;
  llamaServerPath: string | null;
  defaultProfile: Profile;
  capabilities: Capabilities | null;
  capabilityError: string | null;
  calibrationSamples: number;
  fittedOverhead: number | null;
}
