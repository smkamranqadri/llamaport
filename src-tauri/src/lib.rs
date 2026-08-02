pub mod catalog;
pub mod download;
pub mod downloads;
pub mod estimate;
pub mod gguf;
pub mod health;
pub mod probe;
pub mod profile;
pub mod runner;
pub mod store;
pub mod sysmem;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Listener, Manager, State, WindowEvent, Wry};

use catalog::{DirInfo, ModelEntry};
use downloads::{DownloadJob, Downloads, Options};
use estimate::Estimate;
use probe::Capabilities;
use profile::Profile;
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
    downloads: Downloads,
    tray: Mutex<Option<TrayHandles>>,
    orphan: Mutex<Vec<Orphan>>,
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
    swap_used_bytes: Option<u64>,
    pressure: sysmem::Pressure,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    models_dir: String,
    llama_server_path: Option<String>,
    downloads: Options,
    capabilities: Option<Capabilities>,
    capability_error: Option<String>,
}

/// Fills in what the user has not chosen: an alias derived from the model's name, and a
/// context clamped to what the file actually supports.
fn resolve(model: &ModelEntry, draft: Option<Profile>) -> Profile {
    let mut resolved = draft.unwrap_or_default();
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
    draft: Option<Profile>,
) -> Result<LaunchPlan, String> {
    let model = state.model(model_id)?;

    let remembered = {
        let config = state.config.lock().expect("config lock");
        config.last_used.get(&model.id).cloned()
    };

    let profile = resolve(&model, draft.or(remembered));

    let estimate = model.metadata.as_ref().and_then(|md| {
        estimate::estimate(
            md,
            model.size_bytes,
            profile.ctx,
            &profile.cache_type_k,
            &profile.cache_type_v,
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

    let memory = PlanMemory {
        installed_bytes: installed,
        used_bytes: used,
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
        capabilities: caps.as_ref().ok().cloned(),
        capability_error: caps.err(),
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
fn runner_stop(state: State<'_, AppState>) -> Result<RunnerSnapshot, String> {
    state.runner.stop()?;
    Ok(state.runner.snapshot())
}

/// Rescans every time: servers appear and vanish outside this app's knowledge.
#[tauri::command]
fn orphan_status(state: State<'_, AppState>) -> Vec<Orphan> {
    let ours = state.runner.snapshot().pid;
    let found = runner::detect_orphans(ours);
    *state.orphan.lock().expect("orphan lock") = found.clone();
    found
}

#[tauri::command]
fn orphan_stop(pid: u32, state: State<'_, AppState>) -> Result<Vec<Orphan>, String> {
    runner::stop_orphan(pid)?;
    Ok(orphan_status(state))
}

#[tauri::command]
fn download_start(url: String, state: State<'_, AppState>) -> Result<Vec<DownloadJob>, String> {
    let dir = state.models_dir();
    let options = {
        let config = state.config.lock().expect("config lock");
        config.downloads.clone()
    };
    state.downloads.start(&url, &dir, &options)
}

#[tauri::command]
fn download_cancel(id: String, state: State<'_, AppState>) -> Result<Vec<DownloadJob>, String> {
    state.downloads.cancel(&id)
}

#[tauri::command]
fn download_status(state: State<'_, AppState>) -> Vec<DownloadJob> {
    state.downloads.snapshot()
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
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Before the pidfile is read or the config is loaded, both of which live there.
            let _ = store::adopt_legacy_config_dir();

            let handle = app.handle().clone();

            let runner = Runner::new(Arc::new(TauriEvents(handle.clone())));
            let downloads = {
                let handle = handle.clone();
                Downloads::new(
                    Arc::new(TauriEvents(handle.clone())),
                    Arc::new(move || refresh_catalog(&handle)),
                )
            };

            app.manage(AppState {
                config: Mutex::new(store::load()),
                models: Mutex::new(Vec::new()),
                caps: Mutex::new(None),
                runner,
                downloads,
                tray: Mutex::new(None),
                orphan: Mutex::new(runner::detect_orphans(None)),
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

            // An unbundled dev binary with a tray icon can start without activating, so
            // the window exists but never comes to the front. Ask explicitly.
            show_main_window(&handle);

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
            set_llama_server_path,
            get_settings,
            app_version,
            runner_status,
            runner_logs,
            reveal_path,
            runner_start,
            runner_stop,
            health_test,
            orphan_status,
            orphan_stop,
            download_start,
            download_cancel,
            download_status,
            download_clear,
            set_download_options,
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
