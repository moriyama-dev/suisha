//! Command line modes that run without a window:
//! `--auto-update` (started by the system timer) and `--self-check`.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::exec::{find_bin, run};
use crate::managers;
use crate::model::ManagerId;
use crate::ops;
use crate::privileged;
use crate::store;

struct Lock(PathBuf);

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Only one automatic update at a time. A lock older than six hours is
/// assumed to be left over from a crash.
fn take_lock() -> Option<Lock> {
    let dir = store::data_dir();
    fs::create_dir_all(&dir).ok()?;
    let path = dir.join("auto-update.lock");
    if let Ok(meta) = fs::metadata(&path) {
        let old = meta
            .modified()
            .ok()
            .and_then(|m| SystemTime::now().duration_since(m).ok())
            .is_some_and(|age| age > Duration::from_secs(6 * 3600));
        if old {
            let _ = fs::remove_file(&path);
        }
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .ok()
        .map(|_| Lock(path))
}

fn pending(managers: &[ManagerId]) -> HashSet<(ManagerId, String)> {
    managers
        .iter()
        .flat_map(|m| managers::updates(*m).unwrap_or_default())
        .filter(|p| p.held.is_empty())
        .filter_map(|p| Some((p.manager?, p.id)))
        .collect()
}

fn texts(lang: &str) -> [&'static str; 4] {
    if lang == "ja" {
        [
            "自動更新が終わりました",
            "件のソフトウェアを更新しました。",
            "一部の更新に失敗しました",
            "Suisha を開いて、更新の記録を確認してください。",
        ]
    } else {
        [
            "Automatic update finished",
            " updates installed.",
            "Some updates failed",
            "Open Suisha to see the update history.",
        ]
    }
}

fn notify(title: &str, body: &str) {
    if cfg!(target_os = "macos") {
        // The texts reach AppleScript as arguments, never as script code.
        if let Some(osascript) = find_bin("osascript") {
            let _ = run(
                &osascript,
                &[
                    "-e",
                    "on run argv",
                    "-e",
                    "display notification (item 2 of argv) with title (item 1 of argv)",
                    "-e",
                    "end run",
                    title,
                    body,
                ],
                Duration::from_secs(20),
            );
        }
        return;
    }
    if let Some(notify_send) = find_bin("notify-send") {
        let _ = run(
            &notify_send,
            &["--app-name=Suisha", "--icon=suisha", title, body],
            Duration::from_secs(20),
        );
    }
}

pub fn auto_update() -> i32 {
    let settings = store::load_settings();
    let a = &settings.auto_update;
    if !a.enabled {
        return 0;
    }
    let Some(_lock) = take_lock() else {
        eprintln!("suisha: another automatic update is running");
        return 0;
    };

    let available = managers::available();
    let chosen: Vec<ManagerId> = available
        .iter()
        .copied()
        .filter(|m| a.managers.is_empty() || a.managers.contains(m))
        // The App Store updates its apps by itself (its own setting); Suisha
        // can only open its updates page, which makes no sense on a timer.
        .filter(|m| *m != ManagerId::Mas)
        .collect();

    // apt and snap only when the user allowed password-less updates (root
    // needs no permission); a timer has nobody to type a password.
    let include_admin = privileged::helpers_installed() && privileged::auto_update_allowed();

    let skipped_admin: Vec<ManagerId> = if include_admin {
        Vec::new()
    } else {
        chosen.iter().copied().filter(|m| m.needs_admin()).collect()
    };
    if !skipped_admin.is_empty() {
        // Leave a note in the history so the user can see why apt and snap
        // were not updated.
        store::append_history(&[store::HistoryEntry {
            time: store::now(),
            trigger: "scheduled".into(),
            action: "update-all".into(),
            managers: skipped_admin.clone(),
            packages: vec![],
            success: false,
            message: if privileged::helpers_installed() {
                "history.adminSkipped".into()
            } else {
                "auto.noHelpers".into()
            },
        }]);
    }

    let before = pending(&chosen);
    let steps = match managers::plan_update_all(&chosen, include_admin, false) {
        Ok(s) => s,
        Err(e) => {
            store::append_history(&[store::HistoryEntry {
                time: store::now(),
                trigger: "scheduled".into(),
                action: "update-all".into(),
                managers: chosen.clone(),
                packages: vec![],
                success: false,
                message: e,
            }]);
            return 1;
        }
    };
    let result = ops::execute(&steps, "scheduled", "update-all", &mut |line| {
        println!("{line}");
    });
    let after = pending(&chosen);
    let installed = before.difference(&after).count();
    // For the journal (`journalctl --user -u suisha-auto-update`).
    if result.success {
        println!("suisha: {installed} updates installed");
    } else {
        eprintln!("suisha: some updates failed; see `suisha history`");
    }

    if a.notify {
        let t = texts(&settings.language);
        if !result.success || !skipped_admin.is_empty() {
            notify(t[2], t[3]);
        } else if installed > 0 {
            notify(t[0], &format!("{installed}{}", t[1]));
        }
    }
    if result.success { 0 } else { 1 }
}

fn count(r: &Result<Vec<crate::model::Package>, String>) -> serde_json::Value {
    match r {
        Ok(list) => list.len().into(),
        Err(e) => serde_json::json!({ "error": e }),
    }
}

/// Update Suisha itself. Started once a day by a timer when the user turned
/// "update Suisha automatically" on.
pub fn self_update() -> i32 {
    let settings = store::load_settings();
    if !settings.self_update_auto {
        return 0;
    }
    if cfg!(target_os = "macos") {
        if crate::self_update::channel() != "brew" {
            eprintln!("suisha: this copy was not installed with Homebrew");
            return 0;
        }
    } else if !privileged::allowed(privileged::Helper::SelfUpdate) {
        // A timer has nobody to type a password.
        eprintln!("suisha: updating itself without a password is not allowed");
        return 0;
    }
    let steps = match crate::self_update::update_steps(false) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("suisha: {e}");
            return 1;
        }
    };
    let before = crate::self_update::installed_version();
    let mut output = Vec::new();
    for step in &steps {
        let out = crate::exec::run_streaming(&step.program, &step.args, &mut |line| {
            println!("{line}");
        });
        output.push(out);
    }
    let success = output.iter().all(|o| o.success);
    let after = crate::self_update::installed_version();
    // Days without a new version are not worth a line in the history.
    if after != before || !success {
        store::append_history(&[store::HistoryEntry {
            time: store::now(),
            trigger: "scheduled".into(),
            action: "update".into(),
            managers: vec![crate::self_update::manager()],
            packages: vec!["suisha".into()],
            success,
            message: output
                .iter()
                .find(|o| !o.success)
                .map(|o| o.tail())
                .unwrap_or_default(),
        }]);
    }
    if after != before && success {
        let (title, body) = if settings.language == "ja" {
            (
                "Suisha を更新しました".to_string(),
                format!("{after} になりました。次に開いたときから新しい版です。"),
            )
        } else {
            (
                "Suisha was updated".to_string(),
                format!("Version {after} is installed. It starts the next time you open Suisha."),
            )
        };
        notify(&title, &body);
    }
    if success { 0 } else { 1 }
}

/// Print what Suisha can see, as JSON. Used by the release checks.
pub fn self_check() -> i32 {
    let mut report = serde_json::Map::new();
    report.insert("version".into(), crate::VERSION.into());
    report.insert("os".into(), crate::setup_file::current_os().into());
    let mut ok = true;
    for m in managers::available() {
        let installed = managers::installed(m);
        let updates = managers::updates(m);
        ok &= installed.is_ok() && updates.is_ok();
        // Searching needs the network, so it is reported but does not fail the check.
        let search = managers::search(m, "git");
        report.insert(
            m.as_str().into(),
            serde_json::json!({
                "installed": count(&installed),
                "updates": count(&updates),
                "searchGit": count(&search),
            }),
        );
    }
    let system = crate::system::status();
    report.insert(
        "system".into(),
        serde_json::json!({
            "restartRequired": system.restart_required,
            "unneeded": system.unneeded.len(),
            "flathubMissing": system.flathub_missing,
        }),
    );
    // How this copy would update itself; needs no network.
    report.insert(
        "selfUpdateChannel".into(),
        crate::self_update::channel().into(),
    );
    // Needs the network too, so it is reported without failing the check.
    report.insert(
        "selfUpdate".into(),
        match crate::self_update::check() {
            Ok(u) => serde_json::json!({ "latest": u.latest, "available": u.available, "channel": u.channel }),
            Err(e) => serde_json::json!({ "error": e }),
        },
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::Value::Object(report)).unwrap_or_default()
    );
    if ok { 0 } else { 1 }
}
