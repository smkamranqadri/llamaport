pub mod activity;
pub mod catalog;
pub mod discover;
pub mod download;
pub mod downloads;
pub mod estimate;
pub mod gguf;
pub mod health;
pub mod hub;
pub mod pi;
pub mod probe;
pub mod profile;
pub mod quant;
pub mod runner;
pub mod speeds;
pub mod store;
pub mod sysmem;
pub mod tune;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{
    AppHandle, Emitter, Listener, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent,
    Wry,
};

use catalog::{DirInfo, ModelEntry};
use downloads::{DownloadJob, Downloads, Options};
use estimate::Estimate;
use probe::Capabilities;
use profile::Profile;
use runner::{EventSink, LaunchSpec, Orphan, RunState, Runner, RunnerSnapshot};
use store::{Appearance, Config};
use tune::Tuner;

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
    tuner: Tuner,
    downloads: Downloads,
    tray: Mutex<Option<TrayHandles>>,
    orphan: Mutex<Vec<Orphan>>,
    activity: activity::Monitor,
    trees: discover::Trees,
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

    /// What a launch must fit inside, off `llama-server --list-devices`. `None` until the
    /// binary has been found, which is a real state on a first run.
    fn device_budget(&self) -> Option<u64> {
        self.capabilities()
            .ok()
            .and_then(|caps| caps.device_budget_mib())
            .map(|mib| mib * 1024 * 1024)
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
            None => Err(
                "llama-server was not found on PATH or in the usual locations. \
                 Install it with `brew install llama.cpp`, or set its path in Settings."
                    .into(),
            ),
        };
        *cached = Some(result.clone());
        result
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchPlan {
    profile: Profile,
    args: Vec<String>,
    command: String,
    estimate: Option<Estimate>,
    /// Whether the installed build can fit unset arguments to memory, and so whether
    /// Auto is a choice the form may offer.
    fit_available: bool,
    /// What a fully offloaded launch has to fit inside, read from the build rather than
    /// computed from installed memory. `None` where it could not be read — the screen
    /// then says the ceiling is unknown, because falling back to `total_memory` is the
    /// defect this replaced.
    device_budget_bytes: Option<u64>,
    total_memory: u64,
    memory: PlanMemory,
    /// Model metadata: the ceiling the file declares.
    max_ctx: Option<u64>,
    port_conflict: Option<runner::PortConflict>,
    capability_error: Option<String>,
}

/// Machine memory as it stands, plus the safety judgement for the launch being
/// previewed. Every field is optional so one unreadable metric does not blank the panel.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanMemory {
    installed_bytes: Option<u64>,
    used_bytes: Option<u64>,
    /// What the machine can hand out now. Available rather than free: macOS counts the
    /// reclaimable inactive queue as neither used nor free, so `free` reads far below
    /// what a new allocation can actually get.
    available_bytes: Option<u64>,
    swap_used_bytes: Option<u64>,
    pressure: sysmem::Pressure,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MachineMemory {
    installed_bytes: Option<u64>,
    available_bytes: Option<u64>,
    /// What a fully offloaded launch has to fit inside. `None` where llama-server has not
    /// been found, or where the build does not report its devices.
    device_budget_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    models_dir: String,
    llama_server_path: Option<String>,
    downloads: Options,
    /// Absent until the user sets them, which is what tells the screen to offer the
    /// built-in values rather than pretending they were chosen.
    launch_defaults: Option<Profile>,
    /// What those built-in values are. Sent rather than restated on the screen: a second
    /// copy of this drifted from the first the moment a default changed, and the screen
    /// went on offering a context the app no longer chooses.
    built_in_defaults: Profile,
    capabilities: Option<Capabilities>,
    capability_error: Option<String>,
    /// Absent until the user picks one: the app follows macOS until then, and the screen
    /// says so rather than showing a choice nobody made.
    appearance: Option<Appearance>,
}

/// Fills in what the user has not chosen: an alias derived from the model's name, and a
/// context clamped to what the file actually supports.
/// `fits` is whether the installed build has a fitter. Without one, Auto is not a
/// choice the app can honour, so it is resolved away here rather than in `args` — the
/// form shows what the launch will do, and a form reading Auto beside a command naming
/// a number is the disagreement this app refuses everywhere else.
fn resolve(model: &ModelEntry, chosen: Profile, fits: bool) -> Profile {
    let mut resolved = chosen;
    if resolved.alias.trim().is_empty() {
        resolved.alias = profile::default_alias(&model.display_name);
    }
    if !fits {
        if resolved.ctx == profile::AUTO_CTX {
            resolved.ctx = profile::DEFAULT_CTX;
        }
        if resolved.ngl == profile::AUTO_NGL {
            resolved.ngl = "all".to_string();
        }
    }
    // Auto carries no number to cap, and 0.min(max) would not be one either.
    if resolved.ctx != profile::AUTO_CTX {
        if let Some(max) = model.metadata.as_ref().and_then(|m| m.context_length) {
            resolved.ctx = resolved.ctx.min(max);
        }
    }
    resolved
}

fn build_plan(
    state: &AppState,
    model_id: &str,
    draft: Option<Profile>,
) -> Result<LaunchPlan, String> {
    let model = state.model(model_id)?;

    let (remembered, defaults) = {
        let config = state.config.lock().expect("config lock");
        (
            config.last_used.get(&model.id).cloned(),
            config.launch_defaults.clone(),
        )
    };

    let caps = state.capabilities();
    let fits = caps.as_ref().map(|c| c.has("--fit")).unwrap_or(false);
    let profile = resolve(&model, profile::seed(draft, remembered, defaults), fits);

    let estimate = model.metadata.as_ref().and_then(|md| {
        estimate::estimate(
            md,
            model.size_bytes,
            profile.ctx,
            &profile.cache_type_k,
            &profile.cache_type_v,
        )
    });

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
    let available = Some(system.available_memory());
    let pressure = sysmem::pressure();

    let memory = PlanMemory {
        installed_bytes: installed,
        used_bytes: used,
        available_bytes: available,
        swap_used_bytes: swap_used,
        pressure,
    };

    Ok(LaunchPlan {
        total_memory: installed.unwrap_or_else(|| system.total_memory()),
        memory,
        port_conflict: runner::inspect_port(&profile.host, profile.port),
        max_ctx: model.metadata.as_ref().and_then(|m| m.context_length),
        profile,
        args,
        command,
        estimate,
        fit_available: fits,
        device_budget_bytes: caps
            .as_ref()
            .ok()
            .and_then(|c| c.device_budget_mib())
            .map(|mib| mib * 1024 * 1024),
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
    Ok(arranged(&state, entries))
}

/// The catalog is stored in scan order and handed to the screen in display order. The
/// stored order is what `model()` resolves against, and starring a model is not a reason
/// to renumber it.
fn arranged(state: &AppState, entries: Vec<ModelEntry>) -> Vec<ModelEntry> {
    let owners: HashMap<String, String> = state
        .downloads
        .snapshot()
        .into_iter()
        .filter_map(|job| Some((job.path, hub::owner_of(&job.url)?)))
        .collect();

    let mut entries = {
        let config = state.config.lock().expect("config lock");
        catalog::arrange(entries, &config.favourites, &config.last_launched)
    };
    for entry in &mut entries {
        entry.owner = owners.get(&entry.path).cloned();
    }
    entries
}

#[tauri::command]
fn model_favourite(
    model_id: String,
    favourite: bool,
    state: State<'_, AppState>,
) -> Result<Vec<ModelEntry>, String> {
    {
        let mut config = state.config.lock().expect("config lock");
        if favourite {
            config.favourites.insert(model_id);
        } else {
            config.favourites.remove(&model_id);
        }
    }
    state.save_config()?;
    let entries = state.models.lock().expect("models lock").clone();
    Ok(arranged(&state, entries))
}

/// Moves every file the model is made of to the Trash, then rescans.
///
/// The star is left alone deliberately: the identity survives the file, so a model deleted
/// and downloaded again comes back starred. A stale favourite costs nothing.
#[tauri::command]
async fn model_delete(
    model_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ModelEntry>, String> {
    let model = state.model(&model_id)?;
    catalog::deletable(&model, state.runner.snapshot().model_id.as_deref())?;

    let files = catalog::files_of(&model);
    tauri::async_runtime::spawn_blocking(move || catalog::trash(&files))
        .await
        .map_err(|e| e.to_string())??;

    let dir = state.models_dir();
    let entries = tauri::async_runtime::spawn_blocking(move || catalog::scan(&dir))
        .await
        .map_err(|e| e.to_string())?;
    *state.models.lock().expect("models lock") = entries.clone();
    Ok(arranged(&state, entries))
}

/// What the machine has, with no model named. The launch plan carries the same readings,
/// but the first-run screen has to size its offer before any model exists to plan for.
///
/// The device budget is the figure that matters and the one that can be missing: it comes
/// from `llama-server --list-devices`, and a first run may not have found the binary yet.
/// Installed memory is the fallback and the screen says which one it is sizing against —
/// they differ by 7 GB on an M2 Pro, which is more than a whole model.
#[tauri::command]
fn machine_memory(state: State<'_, AppState>) -> MachineMemory {
    let mut system = sysinfo::System::new();
    system.refresh_memory();

    MachineMemory {
        installed_bytes: sysmem::installed_bytes().or_else(|| Some(system.total_memory())),
        available_bytes: Some(system.available_memory()),
        device_budget_bytes: state.device_budget(),
    }
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
    state.downloads.adopt(&dir);
    tauri::async_runtime::spawn_blocking(move || catalog::dir_info(&dir))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn launch_plan(
    model_id: String,
    draft: Option<Profile>,
    state: State<'_, AppState>,
) -> Result<LaunchPlan, String> {
    build_plan(&state, &model_id, draft)
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
        downloads: config.downloads.clone(),
        launch_defaults: config.launch_defaults.clone(),
        built_in_defaults: Profile::default(),
        capabilities: caps.as_ref().ok().cloned(),
        capability_error: caps.err(),
        appearance: config.appearance.clone(),
    }
}

#[tauri::command]
fn set_download_options(options: Options, state: State<'_, AppState>) -> Result<Settings, String> {
    let rate_limit = options.rate_limit;
    {
        let mut config = state.config.lock().expect("config lock");
        config.downloads = options;
    }
    state.save_config()?;
    // A limit is changed while watching the transfer it is meant for, not before the next.
    state.downloads.set_rate_limit(rate_limit);
    Ok(settings_view(&state))
}

/// Where a model that has never been launched opens its form. `None` clears them back to
/// the built-in values.
///
/// Nothing already remembered is touched: these seed a model with no history, and
/// rewriting what a model was actually launched with would be a different feature and a
/// worse one.
#[tauri::command]
fn set_launch_defaults(
    defaults: Option<Profile>,
    state: State<'_, AppState>,
) -> Result<Settings, String> {
    {
        let mut config = state.config.lock().expect("config lock");
        config.launch_defaults = defaults;
    }
    state.save_config()?;
    Ok(settings_view(&state))
}

/// What the app is costing the machine: a row per `llama-server` it knows about, and the
/// machine-wide figures under them.
///
/// Polled by the screen rather than pushed, because it is only ever read while that screen
/// is open — and a CPU figure is a difference between two polls, so the interval the
/// screen chooses is the window it measures over.
#[tauri::command]
fn activity_snapshot(state: State<'_, AppState>) -> activity::Activity {
    let snapshot = state.runner.snapshot();
    let tune = state.tuner.report();
    let tune_pid = state.tuner.live_pid();
    // The scan excludes the app's own child; the measurement's server is the app's too,
    // and is excluded inside `known_processes` rather than here.
    let orphans = runner::detect_orphans(snapshot.pid);
    let known = activity::known_processes(&snapshot, &tune, tune_pid, &orphans);

    let mut activity = state.activity.machine();
    let (processes, total_cpu) = state.activity.poll(&known);
    activity.processes = processes;
    activity.total_cpu_percent = total_cpu;
    activity.device_budget_bytes = state.device_budget();

    // What the running launch asks of the GPU, priced at the context the server actually
    // accepted rather than at Auto, which names no number at all.
    if let Some(model_id) = snapshot.model_id.as_deref() {
        let draft = {
            let config = state.config.lock().expect("config lock");
            config.last_used.get(model_id).cloned()
        };
        let draft = draft.map(|mut profile| {
            if let Some(ctx) = snapshot.server_ctx {
                profile.ctx = ctx;
            }
            profile
        });
        activity.gpu_wanted_bytes = build_plan(&state, model_id, draft)
            .ok()
            .and_then(|plan| plan.estimate)
            .map(|estimate| estimate.total_bytes);
    }

    activity
}

/// The palette and, for the built-in one, whether it follows macOS. Stored verbatim: the
/// names live on the screen that draws them, and this side neither knows nor checks them.
#[tauri::command]
fn set_appearance(appearance: Appearance, state: State<'_, AppState>) -> Result<Settings, String> {
    {
        let mut config = state.config.lock().expect("config lock");
        config.appearance = Some(appearance);
    }
    state.save_config()?;
    Ok(settings_view(&state))
}

/// Read from the bundle rather than `CARGO_PKG_VERSION`, so what a tester quotes is the
/// same string as the `.dmg` they downloaded.
#[tauri::command]
fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    settings_view(&state)
}

#[tauri::command]
fn runner_status(state: State<'_, AppState>) -> RunnerSnapshot {
    state.runner.snapshot()
}

/// Falls back to the previous run's file when memory is empty, so the output that
/// explains a crash survives the app restarting.
#[tauri::command]
fn runner_logs(state: State<'_, AppState>) -> Vec<String> {
    let live = state.runner.logs();
    if live.is_empty() {
        runner::previous_logs()
    } else {
        live
    }
}

const WEBUI_WINDOW: &str = "webui";

/// Opens `llama-server`'s own web UI in a second window.
///
/// A top-level navigation rather than a frame inside the main window: the app document is
/// served from a custom scheme, and whether WKWebView would carry an http subresource
/// inside it is unverified.
#[tauri::command]
fn open_webui_window(app: AppHandle, port: u16) -> Result<(), String> {
    let url = tauri::Url::parse(&format!("http://127.0.0.1:{port}")).map_err(|e| e.to_string())?;

    if let Some(window) = app.get_webview_window(WEBUI_WINDOW) {
        window.navigate(url).map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        return window.set_focus().map_err(|e| e.to_string());
    }

    WebviewWindowBuilder::new(&app, WEBUI_WINDOW, WebviewUrl::External(url))
        .title("llama.cpp — Web UI")
        .inner_size(920.0, 720.0)
        .min_inner_size(480.0, 400.0)
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Driven from the explicit stops rather than from the `runner:state` stream, because
/// `start` stops before it spawns: a listener closing on Idle would tear the window down
/// on every Reload, when the server is about to come back on the same port.
fn close_webui_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(WEBUI_WINDOW) {
        let _ = window.close();
    }
}

/// Selects the file in Finder rather than opening it — a 20 GB GGUF should not be
/// handed to whatever application claims the extension.
#[tauri::command]
fn reveal_path(path: String) -> Result<(), String> {
    std::process::Command::new("open")
        .arg("-R")
        .arg(&path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn runner_start(
    model_id: String,
    draft: Option<Profile>,
    state: State<'_, AppState>,
) -> Result<RunnerSnapshot, String> {
    let model = state.model(&model_id)?;
    if model.error.is_some() {
        return Err("this file could not be parsed as GGUF".into());
    }
    if model.shards.as_ref().is_some_and(|s| !s.missing.is_empty()) {
        return Err("this shard set is incomplete".into());
    }

    // Refuse rather than start a second copy: this app runs one model at a time, and a
    // duplicate silently consumes another 15-20 GB.
    let already = runner::detect_orphans(state.runner.snapshot().pid)
        .into_iter()
        .find(|orphan| orphan.model.as_deref() == Some(model.file_name.as_str()));
    if let Some(existing) = already {
        return Err(match existing.port {
            Some(port) => format!(
                "{} is already running on port {port} (pid {}). Stop it before starting another copy.",
                model.file_name, existing.pid
            ),
            None => format!(
                "{} is already running (pid {}). Stop it before starting another copy.",
                model.file_name, existing.pid
            ),
        });
    }

    let plan = build_plan(&state, &model_id, draft)?;
    if let Some(error) = plan.capability_error {
        return Err(error);
    }
    // Checked on the resolved profile, not the draft: a remembered profile written by an
    // older build can carry raw arguments this one refuses.
    plan.profile.check_raw_args()?;

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
        speed_key: speeds::SpeedKey::of(&model_id, &plan.profile),
        llama_version: caps.version.clone(),
    };

    state.runner.start(spec)?;

    // Remembered only once a launch actually succeeded: settings that failed to start
    // are not what the user wants to come back to.
    {
        let mut config = state.config.lock().expect("config lock");
        config
            .last_used
            .insert(model_id.clone(), plan.profile.clone());
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
    };

    tauri::async_runtime::spawn_blocking(move || health::run(&target))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn runner_stop(app: AppHandle, state: State<'_, AppState>) -> Result<RunnerSnapshot, String> {
    state.runner.stop()?;
    close_webui_window(&app);
    Ok(state.runner.snapshot())
}

/// Rescans every time: servers appear and vanish outside this app's knowledge.
#[tauri::command]
fn orphan_status(state: State<'_, AppState>) -> Vec<Orphan> {
    let ours = state.runner.snapshot().pid;
    // A server Tune is measuring right now is not something the user left behind.
    let measuring = state.tuner.live_pid();
    let found: Vec<Orphan> = runner::detect_orphans(ours)
        .into_iter()
        .filter(|orphan| Some(orphan.pid) != measuring)
        .collect();
    *state.orphan.lock().expect("orphan lock") = found.clone();
    found
}

/// What this model has been seen to do, ranked where the runs earned it.
#[tauri::command]
fn speeds_for(model_id: String, state: State<'_, AppState>) -> speeds::Summary {
    let build = state.capabilities().ok().and_then(|caps| caps.version);
    let records: Vec<speeds::SpeedRecord> = store::load_speeds(&store::speeds_path())
        .into_iter()
        .filter(|record| record.key.model_id == model_id)
        .collect();
    speeds::summarise(&records, build.as_deref())
}

/// What pi would be told, from the server that is actually answering.
///
/// Ready and nothing earlier: the port and the context both come from a server that has
/// replied, and `--fit` is free to give a context the launch never asked for.
fn serving(state: &State<'_, AppState>) -> Result<pi::Serving, String> {
    let snapshot = state.runner.snapshot();
    if snapshot.state != RunState::Ready {
        return Err("no model is serving — pi needs a port that answers".into());
    }
    let (Some(alias), Some(name), Some(port)) =
        (snapshot.alias, snapshot.model_name, snapshot.port)
    else {
        return Err("the running model has no alias, name or port to give pi".into());
    };
    let ctx = snapshot
        .server_ctx
        .ok_or("the server has not reported its context yet")?;

    Ok(pi::Serving {
        alias,
        name,
        port,
        ctx,
    })
}

/// What stands in pi's file, and what confirming would put there.
#[tauri::command]
fn pi_preview(state: State<'_, AppState>) -> Result<pi::Preview, String> {
    pi::preview(
        &store::pi_models_path(),
        &store::pi_settings_path(),
        &serving(&state)?,
    )
}

/// Replaces one provider and writes the rest of the file back untouched.
#[tauri::command]
fn pi_apply(reasoning: bool, state: State<'_, AppState>) -> Result<pi::Preview, String> {
    pi::apply(
        &store::pi_models_path(),
        &store::pi_settings_path(),
        &serving(&state)?,
        reasoning,
    )
}

/// What the app would measure, and why it might refuse to.
#[tauri::command]
fn tune_status(state: State<'_, AppState>) -> tune::Report {
    state.tuner.report()
}

/// Launches each candidate in turn and times it. Refuses while a model is running: this
/// app runs one at a time, and a measurement is not worth stopping a server someone is
/// talking to.
#[tauri::command]
fn tune_start(model_id: String, state: State<'_, AppState>) -> Result<tune::Report, String> {
    if state.runner.snapshot().state != RunState::Idle {
        return Err(
            "stop the running model first — Tune launches servers of its own and this app \
             runs one model at a time"
                .into(),
        );
    }

    let model = state.model(&model_id)?;
    let caps = state.capabilities()?;
    let plan = build_plan(&state, &model_id, None)?;
    plan.profile.check_raw_args()?;

    let metadata = model
        .metadata
        .as_ref()
        .ok_or("this model's header could not be read, so nothing can be sized")?;
    let candidates = tune::candidates(metadata, model.size_bytes, &caps);

    state.tuner.start(tune::Request {
        model_id,
        model_name: model.display_name.clone(),
        model_path: model.path.clone(),
        base: plan.profile,
        candidates,
        llama_version: caps.version.clone(),
        caps,
    })?;
    Ok(state.tuner.report())
}

#[tauri::command]
fn tune_cancel(state: State<'_, AppState>) -> tune::Report {
    state.tuner.cancel();
    state.tuner.report()
}

#[tauri::command]
fn orphan_stop(pid: u32, state: State<'_, AppState>) -> Result<Vec<Orphan>, String> {
    runner::stop_orphan(pid)?;
    Ok(orphan_status(state))
}

#[tauri::command]
fn download_start(url: String, state: State<'_, AppState>) -> Result<Vec<DownloadJob>, String> {
    let dir = state.models_dir();
    // The first run offers a download before the models directory necessarily exists, and
    // every later step — admission, `.part`, room check — assumes a directory to write in.
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let options = {
        let config = state.config.lock().expect("config lock");
        config.downloads.clone()
    };
    state.downloads.start(&url, &dir, &options)
}

/// One listing call and a file tree for each row that needs one. Slow by the standards of
/// this app's other commands — a couple of seconds — because the sizes exist nowhere else.
///
/// **`async` is load-bearing and not decoration.** Tauri runs a synchronous command on the
/// main thread, so this one held it for the whole round trip and the window could not paint
/// the loading state it had already been told to show. Every command that touches the
/// network belongs off that thread.
#[tauri::command]
async fn discover_browse(
    sort: hub::Sort,
    search: Option<String>,
    cursor: Option<String>,
    min_params: Option<u32>,
    max_params: Option<u32>,
    only_moe: bool,
    state: State<'_, AppState>,
) -> Result<discover::Page, String> {
    let params = match (min_params, max_params) {
        (None, None) => None,
        band => Some(band),
    };
    discover::browse(
        &state.trees,
        sort,
        search,
        cursor,
        &discover::Narrow { params, only_moe },
        state.device_budget(),
    )
}

/// The owner's picture, as a `data:` URI. Asked for one owner at a time by the window,
/// rather than being carried on every row: a page of twenty-four has about fifteen distinct
/// owners and the same ones recur, so this is nearly always a cache hit after the first page.
#[tauri::command]
async fn discover_avatar(
    owner: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    Ok(state.trees.avatar(&owner))
}

/// One repository: the facts it states, and every quantisation with its own verdict. The
/// tree is nearly always the one the browsed row already fetched, but the facts call is
/// always made, so this goes off the main thread for the same reason as the browse.
#[tauri::command]
async fn discover_repo(
    repo: String,
    state: State<'_, AppState>,
) -> Result<discover::Detail, String> {
    discover::detail(&state.trees, &repo, state.device_budget())
}

/// Hands the chosen quantisation to the queue that already exists. A shard set is several
/// files and becomes several jobs, which is what the queue was built for.
///
/// Every URL is built by `hub::download_url` and then admitted by `download_start`'s own
/// path, so `downloads::file_name_for` stays the only thing deciding what may land in the
/// models directory.
#[tauri::command]
fn discover_download(
    repo: String,
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<DownloadJob>, String> {
    let urls: Vec<String> = paths
        .iter()
        .map(|path| hub::download_url(&repo, path))
        .collect::<Result<_, _>>()?;

    let mut jobs = state.downloads.snapshot();
    for url in urls {
        jobs = download_start(url, state.clone())?;
    }
    Ok(jobs)
}

#[tauri::command]
fn download_pause(id: String, state: State<'_, AppState>) -> Result<Vec<DownloadJob>, String> {
    state.downloads.pause(&id)
}

#[tauri::command]
fn download_resume(id: String, state: State<'_, AppState>) -> Result<Vec<DownloadJob>, String> {
    let dir = state.models_dir();
    let options = {
        let config = state.config.lock().expect("config lock");
        config.downloads.clone()
    };
    state.downloads.resume(&id, &dir, &options)
}

#[tauri::command]
fn download_discard(id: String, state: State<'_, AppState>) -> Result<Vec<DownloadJob>, String> {
    state.downloads.discard(&id)
}

/// Rescans for partials as well as reporting: a `.part` can appear in the models directory
/// while the app is running — left by an older build, or copied in — and the screen asking
/// for status is the moment the user is looking for it.
#[tauri::command]
fn download_status(state: State<'_, AppState>) -> Vec<DownloadJob> {
    state.downloads.adopt(&state.models_dir())
}

#[tauri::command]
fn download_clear(state: State<'_, AppState>) -> Vec<DownloadJob> {
    state.downloads.clear()
}

/// Rescans after a download lands rather than waiting for the screen to ask, so the new
/// file is launchable from the moment it exists — `runner_start` resolves the model
/// against this list, and a screen that never refreshed would leave it unreachable.
///
/// Runs on the download's own thread, which is already off the main one.
fn refresh_catalog(app: &AppHandle) {
    let state = app.state::<AppState>();
    let entries = catalog::scan(&state.models_dir());
    *state.models.lock().expect("models lock") = entries.clone();
    let _ = app.emit("catalog:changed", entries);
}

/// Dates a model from the run that actually served, for the Library to sort and show.
///
/// Not written where `last_used` is, beside the launch: `runner.start` returns as soon as
/// the process exists, so a model that dies loading its weights would be dated as though
/// it had run. Ready is the first moment that claim is true. It also arrives on every
/// telemetry tick, which is what `stamp_if_newer` is for.
fn record_launch(app: &AppHandle, snapshot: &RunnerSnapshot) {
    if snapshot.state != RunState::Ready {
        return;
    }
    let (Some(model_id), Some(started)) = (snapshot.model_id.as_deref(), snapshot.started_secs)
    else {
        return;
    };

    let state = app.state::<AppState>();
    let stamped = {
        let mut config = state.config.lock().expect("config lock");
        store::stamp_if_newer(&mut config.last_launched, model_id, started)
    };
    // Outside the block on purpose: `save_config` takes the same lock, and it is not
    // reentrant, so saving while holding it deadlocks every launch.
    if stamped {
        let _ = state.save_config();
    }
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

/// Brings the window back to a usable state.
///
/// An unbundled dev binary with a tray icon can start without activating, and macOS may
/// restore a frame far smaller than the configured minimum — observed at 91x97, which is
/// indistinguishable from the app having no window at all. Assert a sane size rather than
/// trusting what was restored.
/// Asserts a usable main window, rebuilding it when there is none.
///
/// Both failures have been seen: no window at all with an empty Window menu, and one at
/// 60x60. Recovery cannot depend on the tray, because someone meeting the app for the
/// first time has no reason to look there.
fn show_main_window(app: &AppHandle) {
    let window = match app.get_webview_window("main") {
        Some(window) => window,
        None => {
            let Some(config) = app.config().app.windows.first().cloned() else {
                return;
            };
            let Ok(builder) = tauri::WebviewWindowBuilder::from_config(app, &config) else {
                return;
            };
            let Ok(window) = builder.build() else {
                return;
            };
            window
        }
    };

    // outer_size is physical and the threshold is logical; on a 2x display comparing them
    // directly calls every window twice the size it is.
    let scale = window.scale_factor().unwrap_or(1.0);
    let too_small = window
        .outer_size()
        .map(|size| size.to_logical::<f64>(scale))
        .map(|size| size.width < 600.0 || size.height < 400.0)
        .unwrap_or(true);

    if too_small {
        let _ = window.set_size(tauri::LogicalSize::new(1060.0, 720.0));
        let _ = window.center();
    }

    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Before the pidfile is read or the config is loaded, both of which live there.
            let _ = store::adopt_legacy_config_dir();

            let handle = app.handle().clone();

            let runner = Runner::new(Arc::new(TauriEvents(handle.clone())));
            let tuner = Tuner::new(Arc::new(TauriEvents(handle.clone())));
            let downloads = {
                let handle = handle.clone();
                Downloads::new(
                    Arc::new(TauriEvents(handle.clone())),
                    Arc::new(move || refresh_catalog(&handle)),
                )
                .persisting_with(Arc::new(|jobs| {
                    let _ = store::save_history(&store::history_path(), jobs);
                }))
            };

            app.manage(AppState {
                config: Mutex::new(store::load()),
                models: Mutex::new(Vec::new()),
                caps: Mutex::new(None),
                runner,
                tuner,
                downloads,
                tray: Mutex::new(None),
                orphan: Mutex::new(runner::detect_orphans(None)),
                activity: activity::Monitor::new(),
                trees: discover::Trees::default(),
            });

            // What finished came from the history file; what did not is on the disk, in
            // the `.part` files a previous run left in the models directory.
            {
                let state = app.state::<AppState>();
                state.downloads.restore(
                    store::load_history(&store::history_path()),
                    &state.models_dir(),
                );
                state.downloads.adopt(&state.models_dir());
            }

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
                        let _ = app.state::<AppState>().runner.stop();
                        close_webui_window(app);
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

            // An unbundled dev binary with a tray icon can start without activating, so
            // the window exists but never comes to the front. Ask explicitly.
            show_main_window(&handle);

            let listener_handle = handle.clone();
            handle.listen("runner:state", move |event| {
                if let Ok(snapshot) = serde_json::from_str::<RunnerSnapshot>(event.payload()) {
                    update_tray(&listener_handle, &snapshot);
                    record_launch(&listener_handle, &snapshot);
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing hides; the app stays in the menu bar so the server survives. Only the
            // main window: a chat window that hides instead of closing would sit invisible
            // and stale, then reappear pointed at a port from a previous run.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            catalog_list,
            machine_memory,
            catalog_dir_info,
            model_favourite,
            model_delete,
            set_models_dir,
            launch_plan,
            set_llama_server_path,
            get_settings,
            app_version,
            runner_status,
            runner_logs,
            reveal_path,
            open_webui_window,
            runner_start,
            runner_stop,
            health_test,
            orphan_status,
            orphan_stop,
            speeds_for,
            tune_status,
            tune_start,
            tune_cancel,
            pi_preview,
            pi_apply,
            download_start,
            discover_browse,
            discover_repo,
            discover_avatar,
            discover_download,
            download_pause,
            download_resume,
            download_discard,
            download_status,
            download_clear,
            set_download_options,
            activity_snapshot,
            set_appearance,
            set_launch_defaults,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
                let state = app.state::<AppState>();
                let _ = state.runner.stop();
                state.tuner.cancel();
            }
            // Closing hides, so without this the Dock icon leads nowhere and the only way
            // back is the tray — which is exactly where a first-time user will not look.
            tauri::RunEvent::Reopen { .. } => show_main_window(app),
            _ => {}
        });
}
