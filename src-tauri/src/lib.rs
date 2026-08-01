pub mod agents;
pub mod benchmarks;
pub mod catalog;
pub mod estimate;
pub mod gguf;
pub mod health;
pub mod probe;
pub mod profile;
pub mod profiles;
pub mod redact;
pub mod runner;
pub mod safety;
pub mod store;
pub mod sysmem;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Listener, Manager, State, WindowEvent, Wry};

use benchmarks::{BenchmarkRecord, Benchmarks};
use catalog::{DirInfo, ModelEntry};
use estimate::Estimate;
use probe::Capabilities;
use profile::{Profile, ProfilePatch};
use profiles::NamedProfile;
use runner::{EventSink, LaunchSpec, Orphan, RunState, Runner, RunnerSnapshot};
use store::Config;

struct TauriEvents(AppHandle);

impl EventSink for TauriEvents {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        let _ = self.0.emit(event, payload);
    }
}

struct TrayHandles {
    status: MenuItem<Wry>,
    stop: MenuItem<Wry>,
}

struct AppState {
    config: Mutex<Config>,
    models: Mutex<Vec<ModelEntry>>,
    caps: Mutex<Option<Result<Capabilities, String>>>,
    runner: Runner,
    tray: Mutex<Option<TrayHandles>>,
    orphan: Mutex<Option<Orphan>>,
    benchmarks: Mutex<Benchmarks>,
}

impl AppState {
    fn models_dir(&self) -> PathBuf {
        let config = self.config.lock().expect("config lock");
        store::models_dir(&config)
    }

    fn save_config(&self) -> Result<(), String> {
        let config = self.config.lock().expect("config lock");
        store::save(&config).map_err(|e| e.to_string())
    }

    fn model(&self, model_id: &str) -> Result<ModelEntry, String> {
        let models = self.models.lock().expect("models lock");
        models
            .iter()
            .find(|m| m.id == model_id)
            .cloned()
            .ok_or_else(|| format!("model {model_id} is not in the catalog"))
    }

    fn capabilities(&self) -> Result<Capabilities, String> {
        let mut cached = self.caps.lock().expect("caps lock");
        if let Some(result) = cached.as_ref() {
            return result.clone();
        }

        let configured = {
            let config = self.config.lock().expect("config lock");
            config.llama_server_path.clone()
        };

        let result = match probe::discover(configured.as_deref()) {
            Some(binary) => probe::probe(&binary),
            None => Err("llama-server was not found on PATH or in the usual locations".into()),
        };
        *cached = Some(result.clone());
        result
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchPlan {
    profile: Profile,
    /// What this model would launch with if it had no overrides — alias derived and
    /// context clamped, so the UI can diff against it directly.
    effective_defaults: Profile,
    overridden: Vec<String>,
    args: Vec<String>,
    command: String,
    estimate: Option<Estimate>,
    total_memory: u64,
    memory: PlanMemory,
    max_ctx: Option<u64>,
    capability_error: Option<String>,
}

/// Machine memory as it stands, plus the safety judgement for the launch being
/// previewed. Every field is optional so one unreadable metric does not blank the panel.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanMemory {
    installed_bytes: Option<u64>,
    used_bytes: Option<u64>,
    swap_used_bytes: Option<u64>,
    pressure: sysmem::Pressure,
    running_model_bytes: Option<u64>,
    assessment: safety::Assessment,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    models_dir: String,
    llama_server_path: Option<String>,
    default_profile: Profile,
    capabilities: Option<Capabilities>,
    capability_error: Option<String>,
    calibration_samples: usize,
    fitted_residency: Option<f64>,
}

fn resolve(defaults: &Profile, model: &ModelEntry, patch: &ProfilePatch) -> Profile {
    let mut resolved = defaults.merged(patch);
    if resolved.alias.trim().is_empty() {
        resolved.alias = profile::default_alias(&model.display_name);
    }
    if let Some(max) = model.metadata.as_ref().and_then(|m| m.context_length) {
        resolved.ctx = resolved.ctx.min(max);
    }
    resolved
}

fn build_plan(
    state: &AppState,
    model_id: &str,
    draft: Option<ProfilePatch>,
) -> Result<LaunchPlan, String> {
    let model = state.model(model_id)?;

    let (defaults, patch, residency) = {
        let config = state.config.lock().expect("config lock");
        (
            config.default_profile.clone(),
            draft.unwrap_or_else(|| config.patch_for(&model.id)),
            estimate::fit_residency(&config.calibration),
        )
    };

    let profile = resolve(&defaults, &model, &patch);
    let effective_defaults = resolve(&defaults, &model, &ProfilePatch::default());
    let overridden = patch
        .overridden_fields()
        .iter()
        .map(|s| s.to_string())
        .collect();

    let estimate = model.metadata.as_ref().and_then(|md| {
        estimate::estimate(
            md,
            model.size_bytes,
            profile.ctx,
            &profile.cache_type_k,
            &profile.cache_type_v,
            residency,
        )
    });

    let caps = state.capabilities();
    let (args, command, capability_error) = match &caps {
        Ok(caps) => {
            let args = profile.args(&model.path, caps);
            let command = profile::render_command(&caps.binary, &args);
            (args, command, None)
        }
        Err(e) => (Vec::new(), String::new(), Some(e.clone())),
    };

    let mut system = sysinfo::System::new();
    system.refresh_memory();

    let installed = sysmem::installed_bytes().or_else(|| Some(system.total_memory()));
    let swap_used = sysmem::swap_used_bytes().or_else(|| Some(system.used_swap()));
    let used = Some(system.used_memory());
    let pressure = sysmem::pressure();
    let running_model_bytes = state.runner.current_model_bytes();

    let memory = PlanMemory {
        installed_bytes: installed,
        used_bytes: used,
        swap_used_bytes: swap_used,
        pressure,
        running_model_bytes,
        assessment: safety::assess(safety::Inputs {
            installed,
            used,
            swap_used,
            pressure,
            running_model_bytes,
            predicted_total: estimate.as_ref().map(|e| e.machine_impact_bytes),
        }),
    };

    Ok(LaunchPlan {
        total_memory: installed.unwrap_or_else(|| system.total_memory()),
        memory,
        max_ctx: model.metadata.as_ref().and_then(|m| m.context_length),
        profile,
        effective_defaults,
        overridden,
        args,
        command,
        estimate,
        capability_error,
    })
}

#[tauri::command]
async fn catalog_list(state: State<'_, AppState>) -> Result<Vec<ModelEntry>, String> {
    let dir = state.models_dir();
    let entries = tauri::async_runtime::spawn_blocking(move || catalog::scan(&dir))
        .await
        .map_err(|e| e.to_string())?;
    *state.models.lock().expect("models lock") = entries.clone();
    Ok(entries)
}

#[tauri::command]
async fn catalog_dir_info(state: State<'_, AppState>) -> Result<DirInfo, String> {
    let dir = state.models_dir();
    tauri::async_runtime::spawn_blocking(move || catalog::dir_info(&dir))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_models_dir(path: String, state: State<'_, AppState>) -> Result<DirInfo, String> {
    {
        let mut config = state.config.lock().expect("config lock");
        config.models_dir = Some(path);
    }
    state.save_config()?;

    let dir = state.models_dir();
    tauri::async_runtime::spawn_blocking(move || catalog::dir_info(&dir))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn launch_plan(
    model_id: String,
    draft: Option<ProfilePatch>,
    state: State<'_, AppState>,
) -> Result<LaunchPlan, String> {
    build_plan(&state, &model_id, draft)
}

#[tauri::command]
fn save_profile(
    model_id: String,
    patch: ProfilePatch,
    state: State<'_, AppState>,
) -> Result<LaunchPlan, String> {
    {
        let mut config = state.config.lock().expect("config lock");
        if patch.overridden_fields().is_empty() {
            config.overrides.remove(&model_id);
        } else {
            config.overrides.insert(model_id.clone(), patch);
        }
    }
    state.save_config()?;
    build_plan(&state, &model_id, None)
}

#[tauri::command]
fn save_default_profile(profile: Profile, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut config = state.config.lock().expect("config lock");
        config.default_profile = profile;
    }
    state.save_config()
}

#[tauri::command]
fn set_llama_server_path(
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<Settings, String> {
    {
        let mut config = state.config.lock().expect("config lock");
        config.llama_server_path = path;
    }
    *state.caps.lock().expect("caps lock") = None;
    state.save_config()?;
    Ok(settings_view(&state))
}

fn settings_view(state: &AppState) -> Settings {
    let caps = state.capabilities();
    let config = state.config.lock().expect("config lock");

    Settings {
        models_dir: store::models_dir(&config).to_string_lossy().into_owned(),
        llama_server_path: config.llama_server_path.clone(),
        default_profile: config.default_profile.clone(),
        calibration_samples: config.calibration.len(),
        fitted_residency: estimate::fit_residency(&config.calibration),
        capabilities: caps.as_ref().ok().cloned(),
        capability_error: caps.err(),
    }
}

#[tauri::command]
fn list_profiles(state: State<'_, AppState>) -> Vec<NamedProfile> {
    let config = state.config.lock().expect("config lock");
    profiles::resolve_all(&config.profiles)
}

/// Creates when `id` is empty, updates otherwise. A colliding name is suffixed rather
/// than rejected, so saving never fails on a name the user cannot see.
#[tauri::command]
fn save_named_profile(
    mut profile: NamedProfile,
    state: State<'_, AppState>,
) -> Result<Vec<NamedProfile>, String> {
    if profile.name.trim().is_empty() {
        return Err("a profile needs a name".into());
    }

    {
        let mut config = state.config.lock().expect("config lock");
        let resolved = profiles::resolve_all(&config.profiles);

        let others: Vec<NamedProfile> = resolved
            .iter()
            .filter(|entry| entry.id != profile.id)
            .cloned()
            .collect();

        profile.name = profiles::unique_name(profile.name.trim(), &others);
        if profile.id.trim().is_empty() {
            profile.id = profiles::new_id(&profile.name, &others);
        }
        profile.built_in = profiles::is_built_in(&profile.id);

        match config
            .profiles
            .iter_mut()
            .find(|entry| entry.id == profile.id)
        {
            Some(existing) => *existing = profile,
            None => config.profiles.push(profile),
        }
    }

    state.save_config()?;
    Ok(list_profiles(state))
}

#[tauri::command]
fn duplicate_profile(id: String, state: State<'_, AppState>) -> Result<Vec<NamedProfile>, String> {
    {
        let mut config = state.config.lock().expect("config lock");
        let resolved = profiles::resolve_all(&config.profiles);

        let source = resolved
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| format!("no profile with id {id}"))?;

        let mut copy = source.clone();
        copy.built_in = false;
        copy.name = profiles::unique_name(&source.name, &resolved);
        copy.id = profiles::new_id(&copy.name, &resolved);
        config.profiles.push(copy);
    }

    state.save_config()?;
    Ok(list_profiles(state))
}

/// Built-in templates cannot be deleted — only reset. Deleting one would leave the app
/// permanently missing a template the UI advertises.
#[tauri::command]
fn delete_profile(id: String, state: State<'_, AppState>) -> Result<Vec<NamedProfile>, String> {
    if profiles::is_built_in(&id) {
        return Err("built-in templates cannot be deleted, only reset".into());
    }

    {
        let mut config = state.config.lock().expect("config lock");
        let before = config.profiles.len();
        config.profiles.retain(|entry| entry.id != id);
        if config.profiles.len() == before {
            return Err(format!("no profile with id {id}"));
        }
    }

    state.save_config()?;
    Ok(list_profiles(state))
}

/// Drops the stored edit so the code definition applies again. User profiles are not
/// touched.
#[tauri::command]
fn reset_profile(id: String, state: State<'_, AppState>) -> Result<Vec<NamedProfile>, String> {
    if !profiles::is_built_in(&id) {
        return Err("only built-in templates can be reset".into());
    }

    {
        let mut config = state.config.lock().expect("config lock");
        config.profiles.retain(|entry| entry.id != id);
    }

    state.save_config()?;
    Ok(list_profiles(state))
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    settings_view(&state)
}

#[tauri::command]
fn runner_status(state: State<'_, AppState>) -> RunnerSnapshot {
    state.runner.snapshot()
}

#[tauri::command]
fn runner_logs(state: State<'_, AppState>) -> Vec<String> {
    state.runner.logs()
}

#[tauri::command]
fn runner_start(
    model_id: String,
    draft: Option<ProfilePatch>,
    state: State<'_, AppState>,
) -> Result<RunnerSnapshot, String> {
    let model = state.model(&model_id)?;
    if model.error.is_some() {
        return Err("this file could not be parsed as GGUF".into());
    }
    if model.shards.as_ref().is_some_and(|s| !s.missing.is_empty()) {
        return Err("this shard set is incomplete".into());
    }

    let plan = build_plan(&state, &model_id, draft)?;
    if let Some(error) = plan.capability_error {
        return Err(error);
    }

    let caps = state.capabilities()?;
    let predicted_base = plan
        .estimate
        .as_ref()
        .map(|e| e.weights_bytes + e.kv_bytes)
        .unwrap_or(0);

    let spec = LaunchSpec {
        model_id: model_id.clone(),
        model_name: model.display_name.clone(),
        binary: caps.binary.clone(),
        args: plan.args,
        alias: plan.profile.alias.clone(),
        host: plan.profile.host.clone(),
        port: plan.profile.port,
        ctx: plan.profile.ctx,
        cache_type_k: plan.profile.cache_type_k.clone(),
        cache_type_v: plan.profile.cache_type_v.clone(),
        predicted_base,
    };

    state.runner.start(spec)?;

    {
        let mut config = state.config.lock().expect("config lock");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        config.last_run.insert(model_id, now);
    }
    let _ = state.save_config();

    Ok(state.runner.snapshot())
}

/// Runs the health checks against whatever is currently running. Blocking on purpose —
/// it is a user-initiated action with a visible result — but off the main thread.
#[tauri::command]
async fn health_test(state: State<'_, AppState>) -> Result<health::HealthReport, String> {
    let snapshot = state.runner.snapshot();
    if snapshot.state != RunState::Ready {
        return Err("start a model before testing it".into());
    }

    let target = health::Target {
        host: "127.0.0.1".to_string(),
        port: snapshot.port.ok_or("no port recorded")?,
        alias: snapshot.alias.unwrap_or_default(),
        pid: snapshot.pid,
        api_key: None,
    };

    tauri::async_runtime::spawn_blocking(move || health::run(&target))
        .await
        .map_err(|e| e.to_string())
}

/// Separate from the health probe on purpose. The probe is instant and shallow; this
/// prefills to a working depth first, because decode speed at an empty context is
/// roughly double what the same model delivers in use, and recording that as a
/// benchmark would be worse than recording nothing.
#[tauri::command]
async fn benchmark_run(
    depth_tokens: u64,
    generate_tokens: u32,
    state: State<'_, AppState>,
) -> Result<BenchmarkRecord, String> {
    let snapshot = state.runner.snapshot();
    if snapshot.state != RunState::Ready {
        return Err("start a model before benchmarking it".into());
    }
    let model_id = snapshot
        .model_id
        .clone()
        .ok_or("no model recorded for the running server")?;

    let target = health::Target {
        host: "127.0.0.1".to_string(),
        port: snapshot.port.ok_or("no port recorded")?,
        alias: snapshot.alias.clone().unwrap_or_default(),
        pid: snapshot.pid,
        api_key: None,
    };

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let spec = health::BenchmarkSpec {
        depth_tokens,
        generate_tokens,
        nonce,
    };

    let started = std::time::Instant::now();
    let outcome = tauri::async_runtime::spawn_blocking(move || health::benchmark(&target, &spec))
        .await
        .map_err(|e| e.to_string())??;

    record_benchmark(
        &state,
        &model_id,
        &outcome,
        started.elapsed().as_millis() as u64,
    )
}

/// A benchmark row is only useful next to the settings that produced it, so the record
/// captures the resolved profile and the llama.cpp build alongside the numbers.
fn record_benchmark(
    state: &AppState,
    model_id: &str,
    outcome: &health::BenchmarkOutcome,
    test_duration_ms: u64,
) -> Result<BenchmarkRecord, String> {
    let model = state.model(model_id)?;
    let plan = build_plan(state, model_id, None)?;

    let (peak_process_bytes, peak_swap_bytes) = state.runner.peaks();
    let timestamp_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let record = BenchmarkRecord {
        id: format!("{timestamp_secs}-{model_id}"),
        timestamp_secs,
        model_file: model.file_name.clone(),
        model_size_bytes: model.size_bytes,
        architecture: model.metadata.as_ref().map(|m| m.architecture.clone()),
        quantisation: model.quant.clone(),
        profile_name: None,
        ctx: plan.profile.ctx,
        cache_type_k: plan.profile.cache_type_k.clone(),
        cache_type_v: plan.profile.cache_type_v.clone(),
        ngl: plan.profile.ngl.clone(),
        parallel: plan.profile.parallel,
        llama_version: state.capabilities().ok().and_then(|caps| caps.version),
        depth_tokens: outcome.depth_tokens,
        time_to_first_token_ms: outcome.time_to_first_token_ms,
        prompt_tokens: outcome.prompt_tokens,
        prompt_tps: outcome.prompt_tps,
        generated_tokens: outcome.generated_tokens,
        gen_tps: outcome.gen_tps,
        peak_process_bytes,
        peak_swap_bytes,
        test_duration_ms,
        verdict: health::Verdict::Passed,
        note: None,
    };

    {
        let mut history = state.benchmarks.lock().expect("benchmarks lock");
        benchmarks::add(&mut history, record.clone());
        benchmarks::save_to(&benchmarks::path(), &history).map_err(|e| e.to_string())?;
    }
    Ok(record)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentConnection {
    connection: Option<agents::Connection>,
    healthy: bool,
    context_tokens: Option<u64>,
    display_name: Option<String>,
    reasoning: bool,
    apps: Vec<agents::DetectedApp>,
    sessions_dir: Option<String>,
}

#[tauri::command]
fn agent_connection(state: State<'_, AppState>) -> AgentConnection {
    let snapshot = state.runner.snapshot();
    let healthy = snapshot.state == RunState::Ready;

    let connection = match (snapshot.port, snapshot.alias.clone()) {
        (Some(port), Some(alias)) if healthy => Some(agents::connection("127.0.0.1", port, &alias)),
        _ => None,
    };

    let model = snapshot
        .model_id
        .as_ref()
        .and_then(|id| state.model(id).ok());

    AgentConnection {
        connection,
        healthy,
        context_tokens: snapshot.server_ctx,
        display_name: model.as_ref().map(|m| match &m.quant {
            Some(quant) => format!("{} {}", m.display_name, quant),
            None => m.display_name.clone(),
        }),
        // Qwen-family models in this library return reasoning; the model test confirms it
        // per server rather than trusting this default.
        reasoning: true,
        apps: agents::detect_apps(),
        sessions_dir: agents::pi_sessions_dir(),
    }
}

/// Reads only when the user asks. Returns structure, never file contents.
#[tauri::command]
fn pi_inspect() -> agents::PiInspection {
    agents::inspect_pi()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PiPreview {
    provider: String,
    settings: String,
    models_path: String,
    settings_path: String,
}

#[tauri::command]
fn pi_preview(
    provider_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<PiPreview, String> {
    let info = agent_connection(state);
    let connection = info
        .connection
        .ok_or("start a model before generating a configuration")?;

    let inspection = agents::inspect_pi();
    let existing = inspection.local_provider.as_ref();
    let provider_name = provider_name
        .or_else(|| existing.map(|p| p.name.clone()))
        .unwrap_or_else(|| "local-llama".to_string());

    let home = std::env::var("HOME").unwrap_or_default();
    Ok(PiPreview {
        provider: agents::pi_provider_preview(&agents::PreviewInput {
            provider_name: provider_name.clone(),
            connection: &connection,
            display_name: info
                .display_name
                .unwrap_or_else(|| connection.alias.clone()),
            context_tokens: info.context_tokens.unwrap_or(4096),
            reasoning: info.reasoning,
            existing,
        }),
        settings: agents::pi_settings_preview(&provider_name, &connection.alias),
        models_path: format!("{home}/.pi/agent/models.json"),
        settings_path: format!("{home}/.pi/agent/settings.json"),
    })
}

#[tauri::command]
fn benchmarks_list(
    query: Option<benchmarks::Query>,
    state: State<'_, AppState>,
) -> Vec<BenchmarkRecord> {
    let history = state.benchmarks.lock().expect("benchmarks lock");
    benchmarks::query(&history.records, &query.unwrap_or_default())
}

#[tauri::command]
fn benchmark_delete(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<BenchmarkRecord>, String> {
    {
        let mut history = state.benchmarks.lock().expect("benchmarks lock");
        if !benchmarks::delete(&mut history, &id) {
            return Err(format!("no benchmark with id {id}"));
        }
        benchmarks::save_to(&benchmarks::path(), &history).map_err(|e| e.to_string())?;
    }
    Ok(benchmarks_list(None, state))
}

#[tauri::command]
fn benchmark_note(
    id: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<BenchmarkRecord>, String> {
    {
        let mut history = state.benchmarks.lock().expect("benchmarks lock");
        if !benchmarks::set_note(&mut history, &id, note) {
            return Err(format!("no benchmark with id {id}"));
        }
        benchmarks::save_to(&benchmarks::path(), &history).map_err(|e| e.to_string())?;
    }
    Ok(benchmarks_list(None, state))
}

/// Writes into the app's own support directory and returns the path, rather than
/// depending on a file dialog plugin.
#[tauri::command]
fn benchmarks_export(format: String, state: State<'_, AppState>) -> Result<String, String> {
    let history = state.benchmarks.lock().expect("benchmarks lock");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let (name, body) = match format.as_str() {
        "csv" => (
            format!("benchmarks-{stamp}.csv"),
            benchmarks::to_csv(&history.records),
        ),
        "json" => (
            format!("benchmarks-{stamp}.json"),
            serde_json::to_string_pretty(&history.records).map_err(|e| e.to_string())?,
        ),
        other => return Err(format!("unsupported export format: {other}")),
    };

    let path = store::config_dir().join("exports").join(name);
    store::write_atomic(&path, &body).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
fn runner_stop(state: State<'_, AppState>) -> Result<RunnerSnapshot, String> {
    state.runner.stop()?;
    Ok(state.runner.snapshot())
}

#[tauri::command]
fn orphan_status(state: State<'_, AppState>) -> Option<Orphan> {
    state.orphan.lock().expect("orphan lock").clone()
}

#[tauri::command]
fn orphan_stop(pid: u32, state: State<'_, AppState>) -> Result<(), String> {
    runner::stop_orphan(pid)?;
    *state.orphan.lock().expect("orphan lock") = None;
    Ok(())
}

#[tauri::command]
fn orphan_dismiss(state: State<'_, AppState>) {
    runner::dismiss_orphan();
    *state.orphan.lock().expect("orphan lock") = None;
}

fn update_tray(app: &AppHandle, snapshot: &RunnerSnapshot) {
    let state = app.state::<AppState>();
    let tray = state.tray.lock().expect("tray lock");
    let Some(handles) = tray.as_ref() else { return };

    let label = match snapshot.state {
        RunState::Idle => "No model running".to_string(),
        RunState::Starting => format!("Starting {}…", snapshot.model_name.as_deref().unwrap_or("")),
        RunState::Ready => format!(
            "{} · :{}",
            snapshot.alias.as_deref().unwrap_or(""),
            snapshot.port.unwrap_or(0)
        ),
        RunState::Stopping => "Stopping…".to_string(),
        RunState::Crashed => "Stopped after a crash".to_string(),
    };

    let _ = handles.status.set_text(label);
    let _ = handles.stop.set_enabled(matches!(
        snapshot.state,
        RunState::Starting | RunState::Ready
    ));
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let sink_handle = handle.clone();

            let runner = Runner::new(
                Arc::new(TauriEvents(handle.clone())),
                Arc::new(move |sample| {
                    let state = sink_handle.state::<AppState>();
                    {
                        let mut config = state.config.lock().expect("config lock");
                        config.record_sample(sample);
                    }
                    let _ = state.save_config();
                }),
            );

            app.manage(AppState {
                config: Mutex::new(store::load()),
                models: Mutex::new(Vec::new()),
                caps: Mutex::new(None),
                runner,
                tray: Mutex::new(None),
                orphan: Mutex::new(runner::detect_orphan()),
                benchmarks: Mutex::new(benchmarks::load_from(&benchmarks::path())),
            });

            let status = MenuItem::with_id(app, "status", "No model running", false, None::<&str>)?;
            let stop = MenuItem::with_id(app, "stop", "Stop model", false, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&status, &stop, &show, &quit])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "stop" => {
                        let state = app.state::<AppState>();
                        let _ = state.runner.stop();
                        let snapshot = state.runner.snapshot();
                        let _ = app.emit("runner:state", snapshot);
                    }
                    "show" => show_main_window(app),
                    "quit" => {
                        let state = app.state::<AppState>();
                        let _ = state.runner.stop();
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            {
                let state = app.state::<AppState>();
                *state.tray.lock().expect("tray lock") = Some(TrayHandles { status, stop });
            }

            let listener_handle = handle.clone();
            handle.listen("runner:state", move |event| {
                if let Ok(snapshot) = serde_json::from_str::<RunnerSnapshot>(event.payload()) {
                    update_tray(&listener_handle, &snapshot);
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing hides; the app stays in the menu bar so the server survives.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            catalog_list,
            catalog_dir_info,
            set_models_dir,
            launch_plan,
            save_profile,
            save_default_profile,
            set_llama_server_path,
            get_settings,
            runner_status,
            runner_logs,
            runner_start,
            runner_stop,
            health_test,
            benchmark_run,
            agent_connection,
            pi_inspect,
            pi_preview,
            benchmarks_list,
            benchmark_delete,
            benchmark_note,
            benchmarks_export,
            orphan_status,
            orphan_stop,
            orphan_dismiss,
            list_profiles,
            save_named_profile,
            duplicate_profile,
            delete_profile,
            reset_profile
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit = event {
                let state = app.state::<AppState>();
                let _ = state.runner.stop();
            }
        });
}
