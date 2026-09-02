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
  slidingWindow: number | null;
  fullAttentionInterval: number | null;
  keyLengthSwa: number | null;
  valueLengthSwa: number | null;
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

export interface MachineMemory {
  installedBytes: number | null;
  availableBytes: number | null;
  /// What a fully offloaded launch must fit inside — absent until llama-server is found.
  deviceBudgetBytes: number | null;
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
  /// True where a term was left out, so both figures are floors.
  bounded: boolean;
  boundNote: string | null;
}

export interface LaunchPlan {
  profile: Profile;
  args: string[];
  command: string;
  estimate: Estimate | null;
  /// Whether the installed build can fit unset arguments to memory, and so whether
  /// the form may offer Auto.
  fitAvailable: boolean;
  /// What a fully offloaded launch must fit inside, read from the build. Null where it
  /// could not be read; the screen then says so rather than using installed memory.
  deviceBudgetBytes: number | null;
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
  /// What the machine can hand out now — available, not free.
  availableBytes: number | null;
  swapUsedBytes: number | null;
  pressure: Pressure;
}

export interface ActivityProcess {
  pid: number;
  name: string;
  /// "model", "measurement" or "stray" — what the row is drawn as.
  kind: string;
  port: number | null;
  memoryBytes: number | null;
  /// Percent of one core, the way Activity Monitor counts it.
  cpuPercent: number | null;
}

export interface Activity {
  processes: ActivityProcess[];
  totalCpuPercent: number | null;
  memoryUsedBytes: number | null;
  memoryTotalBytes: number | null;
  swapUsedBytes: number | null;
  pressure: Pressure;
  /// What a launch must fit inside. Null where llama-server has not been found.
  deviceBudgetBytes: number | null;
  /// What the running model's launch asks of that, null when nothing runs.
  gpuWantedBytes: number | null;
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

/// The palette, and whether the built-in one follows macOS. Both are plain strings on
/// purpose: a name written by a later build is kept rather than dropped, and the screen
/// falls back to the default when it does not recognise one.
export interface Appearance {
  theme: string;
  mode: string;
}

export interface Settings {
  modelsDir: string;
  llamaServerPath: string | null;
  downloads: DownloadOptions;
  /// Null until the user sets them: the built-in values are then in force.
  launchDefaults: Profile | null;
  /// What those built-in values are, so the screen never restates them.
  builtInDefaults: Profile;
  capabilities: Capabilities | null;
  capabilityError: string | null;
  /// Null until the user picks one: the app follows macOS until then.
  appearance: Appearance | null;
}

export interface TuneCandidate {
  ctx: number;
  cacheK: string;
  cacheV: string;
}

export interface TuneReading {
  promptTokens: number;
  promptSeconds: number;
  genTokens: number;
  genSeconds: number;
}

/// A failure is a result too: "this did not load" answers the question the ladder asked.
export interface TuneOutcome {
  candidate: TuneCandidate;
  reading: TuneReading | null;
  error: string | null;
}

export interface TuneReport {
  running: boolean;
  modelId: string | null;
  modelName: string | null;
  promptWords: number | null;
  candidates: TuneCandidate[];
  current: TuneCandidate | null;
  done: number;
  total: number;
  rows: TuneOutcome[];
  error: string | null;
  cancelled: boolean;
}

export type SpeedSource = "observed" | "measured";
export type SpeedConfidence = "neverMeasured" | "observed" | "tuned";

export interface SpeedKey {
  modelId: string;
  ctx: number;
  ngl: string;
  cacheTypeK: string;
  cacheTypeV: string;
  flashAttn: boolean;
  parallel: number;
  rawArgs: string[];
}

export interface SpeedRow {
  key: SpeedKey;
  source: SpeedSource;
  genTps: number | null;
  promptTps: number | null;
  promptTokens: number;
  genTokens: number;
  timestampSecs: number;
  llamaVersion: string | null;
  stale: boolean;
  ranked: boolean;
  runs: number;
}

export interface SpeedSummary {
  rows: SpeedRow[];
  confidence: SpeedConfidence;
  suggestion: SpeedKey | null;
  suggestedTps: number | null;
  tied: number;
  beats: SpeedKey | null;
  beatsByPercent: number | null;
}

export interface PiFileChange {
  path: string;
  before: string | null;
  after: string;
  createsFile: boolean;
}

/// What pi's two files hold for us now, and what confirming would put there.
export interface PiPreview {
  provider: PiFileChange;
  enabled: PiFileChange;
  pruned: string[];
  sharingPort: string[];
  reasoning: boolean;
}
