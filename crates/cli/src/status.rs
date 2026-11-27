//! `status` and `history`.

use serde::Serialize;
use suisha_core::managers;
use suisha_core::model::ManagerId;
use suisha_core::setup_file;
use suisha_core::store::{self, HistoryEntry};
use suisha_core::system::{self, SystemStatus};

use crate::args::Args;
use crate::autostart;
use crate::packages::chosen;
use crate::run::print_json;
use crate::schedule;
use crate::text::{self, count, table, tr};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManagerStatus {
    manager: ManagerId,
    installed: Option<usize>,
    /// Updates that can be installed now.
    updates: Option<usize>,
    /// Updates apt holds back for now (phased or kept back).
    held: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    version: &'static str,
    os: &'static str,
    managers: Vec<ManagerStatus>,
    auto_update: schedule::Info,
    startup_check: autostart::Info,
    system: SystemStatus,
}

fn manager_status(m: ManagerId) -> ManagerStatus {
    let installed = managers::installed(m);
    let updates = managers::updates(m);
    let error = installed.as_ref().err().or(updates.as_ref().err()).cloned();
    let held = updates
        .as_ref()
        .map(|list| list.iter().filter(|p| !p.held.is_empty()).count())
        .unwrap_or(0);
    ManagerStatus {
        manager: m,
        installed: installed.ok().map(|list| list.len()),
        updates: updates.ok().map(|list| list.len() - held),
        held,
        error,
    }
}

pub fn status(args: &Args) -> Result<i32, String> {
    let status = Status {
        version: suisha_core::VERSION,
        os: setup_file::current_os(),
        managers: chosen(args)?.into_iter().map(manager_status).collect(),
        auto_update: schedule::info(),
        startup_check: autostart::info(),
        system: system::status(),
    };
    let failed = status.managers.iter().any(|m| m.error.is_some());
    if args.json {
        print_json(&status);
        return Ok(i32::from(failed));
    }

    println!("Suisha {}", status.version);
    println!();
    println!("{}", tr!("Updates", "更新"));
    let rows: Vec<Vec<String>> = status
        .managers
        .iter()
        .map(|m| {
            let updates = match (&m.error, m.updates) {
                (Some(e), _) => tr!("could not be read: {e}", "読み取れませんでした：{e}"),
                (None, Some(0) | None) => tr!("up to date", "最新です"),
                (None, Some(n)) => {
                    let waiting = count(n, "update", "updates");
                    tr!("{waiting} waiting", "{n} 件の更新")
                }
            };
            let held = match m.held {
                0 => String::new(),
                n => tr!(
                    "{n} more held back by apt for now",
                    "ほかに {n} 件は apt が保留中"
                ),
            };
            let installed = m
                .installed
                .map(|n| tr!("{n} installed", "{n} 件インストール済み"))
                .unwrap_or_default();
            vec![
                format!("  {}", m.manager.as_str()),
                updates,
                installed,
                held,
            ]
        })
        .collect();
    print!("{}", table(&rows));
    if status.managers.iter().any(|m| m.updates.unwrap_or(0) > 0) {
        println!(
            "  {}",
            tr!(
                "Install them with: suisha update",
                "入れるには：suisha update"
            )
        );
    }

    println!();
    println!("{}", tr!("Automatic updates", "自動更新"));
    for line in schedule::lines(&status.auto_update) {
        println!("  {line}");
    }

    println!();
    println!("{}", tr!("Check at startup", "起動時の確認"));
    for line in autostart::lines(&status.startup_check) {
        println!("  {line}");
    }

    let system = &status.system;
    if system.restart_required || !system.unneeded.is_empty() || system.flathub_missing {
        println!();
        println!("{}", tr!("System", "システム"));
    }
    if system.restart_required {
        if system.restart_packages.is_empty() {
            println!(
                "  {}",
                tr!(
                    "A restart is needed to finish installing updates.",
                    "更新を完了するには再起動が必要です。"
                )
            );
        } else {
            let list = system.restart_packages.join(", ");
            println!(
                "  {}",
                tr!(
                    "A restart is needed to finish installing these updates: {list}",
                    "次の更新を完了するには再起動が必要です：{list}"
                )
            );
        }
    }
    if !system.unneeded.is_empty() {
        let n = system.unneeded.len();
        let packages = count(n, "package is", "packages are");
        println!(
            "  {}",
            tr!(
                "{packages} no longer needed. Remove with: suisha cleanup",
                "不要になったパッケージが {n} 件あります。削除するには：suisha cleanup"
            )
        );
    }
    if system.flathub_missing {
        println!(
            "  {}",
            tr!(
                "Flatpak is installed without Flathub, where most Flatpak apps come from.",
                "Flatpak は入っていますが、ほとんどのアプリの入手先である Flathub が登録されていません。"
            )
        );
    }
    Ok(i32::from(failed))
}

fn action_name(action: &str) -> String {
    match action {
        "install" => tr!("install", "インストール"),
        "remove" => tr!("remove", "削除"),
        "update" => tr!("update", "更新"),
        "update-all" => tr!("update all", "すべて更新"),
        "clean-up" => tr!("clean up", "掃除"),
        "add-flathub" => tr!("add Flathub", "Flathub を追加"),
        other => other.to_string(),
    }
}

fn history_row(e: &HistoryEntry) -> Vec<String> {
    let trigger = if e.trigger == "scheduled" {
        tr!("automatic", "自動")
    } else {
        tr!("manual", "手動")
    };
    let result = if e.success {
        tr!("done", "完了")
    } else if e.message == "cancelled" {
        tr!("cancelled", "キャンセル")
    } else {
        tr!("FAILED", "失敗")
    };
    let managers: Vec<&str> = e.managers.iter().map(|m| m.as_str()).collect();
    vec![
        text::local_time(&e.time),
        trigger,
        action_name(&e.action),
        result,
        managers.join(","),
        e.packages.join(" "),
    ]
}

pub fn history(args: &Args) -> Result<i32, String> {
    store::prune_history(store::load_settings().history_keep_days);
    let mut entries = store::read_history();
    entries.truncate(args.limit.unwrap_or(20));
    if args.json {
        print_json(&entries);
        return Ok(0);
    }
    if entries.is_empty() {
        println!(
            "{}",
            tr!(
                "Nothing has been changed yet.",
                "まだ何も変更していません。"
            )
        );
        return Ok(0);
    }
    let rows: Vec<Vec<String>> = entries.iter().map(history_row).collect();
    for (line, e) in table(&rows).lines().zip(&entries) {
        println!("{line}");
        if !e.success && !e.message.is_empty() && e.message != "cancelled" {
            for line in text::message(&e.message).lines() {
                println!("    {line}");
            }
        }
    }
    Ok(0)
}
