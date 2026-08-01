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

export type Workload =
  | "qualityCoding"
  | "balanced"
  | "longContext"
  | "lightweight"
  | "custom";

export interface NamedProfile {
  id: string;
  name: string;
  description: string;
  builtIn: boolean;
  workload: Workload;
  modelId: string | null;
  apiKeyRef: string | null;
  settings: ProfilePatch;
}

export interface Estimate {
  weightsBytes: number;
  kvBytes: number;
  overheadBytes: number;
  totalBytes: number;
  machineImpactBytes: number;
  residency: number | null;
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
  memory: PlanMemory;
  maxCtx: number | null;
  practicalCtx: number | null;
  riskyCtx: number | null;
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
  requestedPort: number | null;
  pid: number | null;
  startedSecs: number | null;
  error: string | null;
  crashTail: string[];
  restarted: boolean;
  serverCtx: number | null;
}

export type Pressure = "normal" | "warning" | "critical" | "unknown";

export type SafetyState = "unknown" | "green" | "yellow" | "red";

export interface Assessment {
  state: SafetyState;
  projectedUsedBytes: number | null;
  headroomBytes: number | null;
  reasons: string[];
}

export interface PlanMemory {
  installedBytes: number | null;
  usedBytes: number | null;
  swapUsedBytes: number | null;
  pressure: Pressure;
  runningModelBytes: number | null;
  assessment: Assessment;
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
  modelDeltaBytes: number | null;
  processFootprintBytes: number | null;
  pressure: Pressure;
  safety: Assessment | null;
  healthOk: boolean;
  uptimeSecs: number;
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

export type BenchmarkSort =
  | "timestamp"
  | "genTps"
  | "promptTps"
  | "timeToFirstToken"
  | "peakMemory";

export interface BenchmarkQuery {
  modelFile: string | null;
  quantisation: string | null;
  sort: BenchmarkSort;
  descending: boolean;
}

export interface BenchmarkRecord {
  id: string;
  timestampSecs: number;
  modelFile: string;
  modelSizeBytes: number;
  architecture: string | null;
  quantisation: string | null;
  profileName: string | null;
  ctx: number;
  cacheTypeK: string;
  cacheTypeV: string;
  ngl: string;
  parallel: number;
  llamaVersion: string | null;
  depthTokens: number | null;
  timeToFirstTokenMs: number | null;
  promptTokens: number | null;
  promptTps: number | null;
  generatedTokens: number | null;
  genTps: number | null;
  peakProcessBytes: number | null;
  peakSwapBytes: number | null;
  testDurationMs: number;
  verdict: Verdict;
  note: string | null;
}

export interface AgentConnectionInfo {
  connection: {
    baseUrl: string;
    openaiUrl: string;
    alias: string;
    host: string;
    port: number;
    loopbackOnly: boolean;
  } | null;
  healthy: boolean;
  contextTokens: number | null;
  displayName: string | null;
  reasoning: boolean;
  apps: { name: string; path: string }[];
  sessionsDir: string | null;
}

export interface PiLocalProvider {
  name: string;
  baseUrl: string | null;
  api: string | null;
  hasApiKey: boolean;
  modelIds: string[];
  extraKeys: string[];
}

export interface PiInspection {
  settingsFound: boolean;
  modelsFound: boolean;
  defaultProvider: string | null;
  defaultModel: string | null;
  providerNames: string[];
  localProvider: PiLocalProvider | null;
  notes: string[];
}

export interface PiPreview {
  provider: string;
  settings: string;
  modelsPath: string;
  settingsPath: string;
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
  fittedResidency: number | null;
}
