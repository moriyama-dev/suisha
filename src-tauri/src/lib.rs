pub mod background;
pub mod exec;
pub mod managers;
pub mod model;
pub mod ops;
pub mod privileged;
pub mod schedule;
pub mod self_update;
pub mod setup_file;
pub mod store;
pub mod system;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use managers::Action;
use model::{ManagerId, Package, PackageRef};
use ops::OpResult;
use store::{HistoryEntry, Settings};

/// Only one change (install, remove, update) runs at a time.
static BUSY: AtomicBool = AtomicBool::new(false);

struct BusyGuard;

impl BusyGuard {
    fn take() -> Result<BusyGuard, String> {
        if BUSY.swap(true, Ordering::SeqCst) {
            Err("busy".into())
        } else {
            Ok(BusyGuard)
        }
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::SeqCst);
    }
}

/// Development builds only: show sample notices, so that every part of the
/// window can be looked at on a machine that has nothing to report.
fn demo() -> bool {
    cfg!(debug_assertions) && std::env::var_os("SUISHA_DEV_DEMO").is_some()
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
    os: String,
    managers: Vec<ManagerId>,
    /// The root helpers from the .deb are installed.
    helpers: bool,
    /// This user may run the automatic update without a password.
    auto_update_allowed: bool,
}

#[tauri::command]
async fn app_info() -> Result<AppInfo, String> {
    blocking(|| {
        Ok(AppInfo {
            version: env!("CARGO_PKG_VERSION").into(),
            os: setup_file::current_os().into(),
            managers: managers::available(),
            helpers: demo() || privileged::helpers_installed(),
            auto_update_allowed: demo() || privileged::auto_update_allowed(),
        })
    })
    .await
}

#[tauri::command]
async fn list_installed(manager: ManagerId) -> Result<Vec<Package>, String> {
    blocking(move || managers::installed(manager)).await
}

#[tauri::command]
async fn list_updates(manager: ManagerId) -> Result<Vec<Package>, String> {
    blocking(move || managers::updates(manager)).await
}

#[tauri::command]
async fn search(manager: ManagerId, query: String) -> Result<Vec<Package>, String> {
    blocking(move || managers::search(manager, &query)).await
}

fn run_steps(app: &AppHandle, steps: &[managers::Step], action: &str) -> OpResult {
    let app = app.clone();
    ops::execute(steps, "manual", action, &mut |line| {
        let _ = app.emit("op-log", line.to_string());
    })
}

#[tauri::command]
async fn run_action(
    app: AppHandle,
    action: Action,
    packages: Vec<PackageRef>,
) -> Result<OpResult, String> {
    blocking(move || {
        let _guard = BusyGuard::take()?;
        let steps = managers::plan(action, &packages)?;
        let name = match action {
            Action::Install => "install",
            Action::Remove => "remove",
            Action::Update => "update",
        };
        Ok(run_steps(&app, &steps, name))
    })
    .await
}

#[tauri::command]
async fn update_all(app: AppHandle, managers: Vec<ManagerId>) -> Result<OpResult, String> {
    blocking(move || {
        let _guard = BusyGuard::take()?;
        let steps = managers::plan_update_all(&managers, true)?;
        Ok(run_steps(&app, &steps, "update-all"))
    })
    .await
}

/// Fetch fresh package information (`brew update`).
#[tauri::command]
async fn refresh(app: AppHandle) -> Result<OpResult, String> {
    blocking(move || {
        let _guard = BusyGuard::take()?;
        Ok(run_steps(&app, &managers::refresh_steps(), "refresh"))
    })
    .await
}

#[tauri::command]
async fn export_setup(path: PathBuf) -> Result<usize, String> {
    blocking(move || {
        let mut all = Vec::new();
        for m in managers::available() {
            all.extend(managers::installed(m)?);
        }
        let file = setup_file::build(&all);
        setup_file::write(&path, &file)?;
        Ok(file.packages.len())
    })
    .await
}

#[tauri::command]
async fn read_setup(path: PathBuf) -> Result<setup_file::Preview, String> {
    blocking(move || {
        let file = setup_file::read(&path)?;
        Ok(setup_file::preview(&file))
    })
    .await
}

#[tauri::command]
fn get_settings() -> Settings {
    store::load_settings()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveResult {
    settings: Settings,
    /// A message key for something the window should explain.
    warning: Option<String>,
}

#[tauri::command]
async fn save_settings(settings: Settings) -> Result<SaveResult, String> {
    blocking(move || {
        if !settings.auto_update.is_valid() {
            return Err("Invalid time".into());
        }
        let previous = store::load_settings();
        let mut warning = None;

        let a = &settings.auto_update;
        let wants_admin = [ManagerId::Apt, ManagerId::Snap].iter().any(|m| {
            managers::binary(*m).is_some() && (a.managers.is_empty() || a.managers.contains(m))
        });
        let turning_on = a.enabled && wants_admin;
        let turning_off = !a.enabled && previous.auto_update.enabled;
        if turning_on && !privileged::auto_update_allowed() {
            if !privileged::helpers_installed() {
                warning = Some("auto.noHelpers".into());
            } else if let Err(e) = privileged::set_auto_update_access(true) {
                warning = Some(if e == "cancelled" {
                    "auto.accessCancelled".into()
                } else {
                    format!("auto.accessFailed:{e}")
                });
            }
        } else if turning_off && privileged::auto_update_allowed() {
            // Taking the permission back is optional; dismissing the
            // dialog leaves it in place, and the timer is gone anyway.
            let _ = privileged::set_auto_update_access(false);
        }

        schedule::apply(&settings.auto_update)?;
        // The card only edits the schedule; keep what it does not know about.
        let settings = Settings {
            self_update_auto: previous.self_update_auto,
            ..settings
        };
        store::save_settings(&settings)?;
        Ok(SaveResult { settings, warning })
    })
    .await
}

/// Let this user's automatic updates update apt and snap without a
/// password. Shows the system password dialog. Returns whether it is allowed now.
#[tauri::command]
async fn grant_auto_update_access() -> Result<bool, String> {
    blocking(|| {
        if !privileged::helpers_installed() {
            return Err("auto.noHelpers".into());
        }
        privileged::set_auto_update_access(true)?;
        Ok(privileged::auto_update_allowed())
    })
    .await
}

#[tauri::command]
async fn check_self_update() -> Result<self_update::SelfUpdate, String> {
    blocking(|| {
        if demo() {
            return Ok(self_update::SelfUpdate {
                current: env!("CARGO_PKG_VERSION").into(),
                latest: "0.5.0".into(),
                available: true,
                channel: "apt".into(),
                releases_url: self_update::RELEASES_URL.into(),
                auto_allowed: false,
            });
        }
        self_update::check()
    })
    .await
}

/// Update Suisha itself now.
#[tauri::command]
async fn update_self(app: AppHandle) -> Result<OpResult, String> {
    blocking(move || {
        let _guard = BusyGuard::take()?;
        let steps = self_update::update_steps()?;
        Ok(run_steps(&app, &steps, "update"))
    })
    .await
}

/// Turn the daily update of Suisha itself on or off. Turning it on asks for
/// the password once, unless that was done before.
#[tauri::command]
async fn set_self_update_auto(enabled: bool) -> Result<Settings, String> {
    blocking(move || {
        if enabled {
            if !privileged::helpers_installed() {
                return Err("auto.noHelpers".into());
            }
            if !privileged::allowed(privileged::Helper::SelfUpdate) {
                privileged::set_access(privileged::Helper::SelfUpdate, true)?;
            }
        }
        schedule::apply_self_update(enabled)?;
        let mut settings = store::load_settings();
        settings.self_update_auto = enabled;
        store::save_settings(&settings)?;
        Ok(settings)
    })
    .await
}

#[tauri::command]
async fn system_status() -> Result<system::SystemStatus, String> {
    blocking(|| {
        if demo() {
            return Ok(system::SystemStatus {
                restart_required: true,
                restart_packages: vec!["linux-base".into(), "linux-image-6.8.0-151-generic".into()],
                unneeded: vec![
                    "linux-headers-6.8.0-149".into(),
                    "linux-headers-6.8.0-149-generic".into(),
                    "linux-tools-6.8.0-149".into(),
                ],
                flathub_missing: true,
            });
        }
        Ok(system::status())
    })
    .await
}

/// Remove packages that are no longer needed (`apt autoremove`).
#[tauri::command]
async fn clean_up(app: AppHandle) -> Result<OpResult, String> {
    blocking(move || {
        let _guard = BusyGuard::take()?;
        let steps = vec![privileged::autoremove_step()?];
        Ok(run_steps(&app, &steps, "clean-up"))
    })
    .await
}

/// Set up Flathub, the app catalogue most Flatpak apps come from.
#[tauri::command]
async fn add_flathub(app: AppHandle) -> Result<OpResult, String> {
    blocking(move || {
        let _guard = BusyGuard::take()?;
        let steps = vec![system::add_flathub_step()?];
        Ok(run_steps(&app, &steps, "add-flathub"))
    })
    .await
}

/// Start the newly installed version.
#[tauri::command]
fn restart_app(app: AppHandle) {
    app.restart();
}

#[tauri::command]
fn set_language(language: String) -> Result<(), String> {
    let mut s = store::load_settings();
    let language = if language == "ja" { "ja" } else { "en" }.to_string();
    if s.language != language {
        s.language = language;
        store::save_settings(&s)?;
    }
    Ok(())
}

#[tauri::command]
fn get_history() -> Vec<HistoryEntry> {
    store::prune_history(store::load_settings().history_keep_days);
    store::read_history()
}

/// How long history entries are kept: a number of days, 0 for no history,
/// or nothing to keep them forever. Older entries are deleted right away.
#[tauri::command]
fn set_history_keep_days(days: Option<u32>) -> Result<Settings, String> {
    if days.is_some_and(|d| d > store::HISTORY_KEEP_DAYS_MAX) {
        return Err("Invalid period".into());
    }
    let mut settings = store::load_settings();
    settings.history_keep_days = days;
    store::save_settings(&settings)?;
    store::prune_history(days);
    Ok(settings)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Keep the timer pointing at this copy of the app (it may have moved).
    let settings = store::load_settings();
    if settings.auto_update.enabled {
        let _ = schedule::apply(&settings.auto_update);
    }
    if settings.self_update_auto {
        let _ = schedule::apply_self_update(true);
    }

    tauri::Builder::default()
        .setup(|_app| {
            // Development builds only: run a script in the window, for
            // screenshots and interaction tests on machines without a desktop.
            #[cfg(debug_assertions)]
            if let Ok(script) = std::env::var("SUISHA_DEV_EVAL") {
                use tauri::Manager;
                if let Some(w) = _app.get_webview_window("main") {
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_secs(4));
                        let _ = w.eval(&script);
                    });
                }
            }
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            app_info,
            list_installed,
            list_updates,
            search,
            run_action,
            update_all,
            refresh,
            export_setup,
            read_setup,
            get_settings,
            save_settings,
            set_language,
            grant_auto_update_access,
            check_self_update,
            update_self,
            set_self_update_auto,
            set_history_keep_days,
            system_status,
            clean_up,
            add_flathub,
            restart_app,
            get_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
