//! Reading state from each package manager, and planning the commands that
//! change it.

pub mod apt;
pub mod brew;
pub mod flatpak;
pub mod mas;
pub mod snap;
pub mod text;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;

use crate::exec::{find_bin, run};
use crate::model::{ManagerId, Package, PackageRef, valid_package_id, valid_simple_word};
use crate::privileged;

const READ_TIMEOUT: Duration = Duration::from_secs(180);
const SEARCH_TIMEOUT: Duration = Duration::from_secs(60);
const SEARCH_LIMIT: usize = 60;

const LINUX: bool = cfg!(target_os = "linux");
const MACOS: bool = cfg!(target_os = "macos");

pub fn binary(m: ManagerId) -> Option<PathBuf> {
    match m {
        ManagerId::Apt if LINUX => find_bin("apt-get"),
        ManagerId::Snap if LINUX => find_bin("snap"),
        ManagerId::Flatpak if LINUX => find_bin("flatpak"),
        // Homebrew on macOS, and on Linux for people who use it next to apt.
        ManagerId::Brew => find_bin("brew"),
        ManagerId::BrewCask if MACOS => find_bin("brew"),
        ManagerId::Mas if MACOS => find_bin("mas"),
        _ => None,
    }
}

/// The macOS `open` command, which hands a link to the app that handles it.
fn open_tool() -> Result<PathBuf, String> {
    tool("open")
}

/// The App Store page of one app.
pub fn app_store_page(id: &str) -> String {
    format!("macappstore://apps.apple.com/app/id{id}")
}

pub const APP_STORE_UPDATES: &str = "macappstore://showUpdatesPage";

pub fn available() -> Vec<ManagerId> {
    ManagerId::ALL
        .into_iter()
        .filter(|m| binary(*m).is_some())
        .collect()
}

fn tool(name: &str) -> Result<PathBuf, String> {
    find_bin(name).ok_or_else(|| format!("{name} was not found"))
}

fn bin(m: ManagerId) -> Result<PathBuf, String> {
    binary(m).ok_or_else(|| format!("{} is not available", m.as_str()))
}

fn checked(out: crate::exec::Output) -> Result<String, String> {
    if out.success {
        Ok(out.stdout)
    } else if out.timed_out {
        Err("The command took too long and was stopped.".into())
    } else {
        Err(out.tail())
    }
}

/// Packages the user installed (dependencies and support packages are left out).
pub fn installed(m: ManagerId) -> Result<Vec<Package>, String> {
    match m {
        ManagerId::Apt => {
            let manual = checked(run(&tool("apt-mark")?, &["showmanual"], READ_TIMEOUT))?;
            let dpkg = checked(run(
                &tool("dpkg-query")?,
                &["-W", "-f", apt::DPKG_FORMAT],
                READ_TIMEOUT,
            ))?;
            Ok(apt::installed_from(&manual, &dpkg))
        }
        ManagerId::Snap => Ok(snap::parse_list(&checked(run(
            &bin(m)?,
            &["list"],
            READ_TIMEOUT,
        ))?)),
        ManagerId::Flatpak => Ok(flatpak::parse_list(&checked(run(
            &bin(m)?,
            &[
                "list",
                "--app",
                "--columns=application,name,version,origin,installation",
            ],
            READ_TIMEOUT,
        ))?)),
        ManagerId::Brew => {
            let brew = bin(m)?;
            let leaves = checked(run(
                &brew,
                &["leaves", "--installed-on-request"],
                READ_TIMEOUT,
            ))?;
            let versions = checked(run(
                &brew,
                &["list", "--formula", "--versions"],
                READ_TIMEOUT,
            ))?;
            Ok(brew::formulae_from(&leaves, &versions))
        }
        ManagerId::BrewCask => Ok(brew::casks_from(&checked(run(
            &bin(m)?,
            &["list", "--cask", "--versions"],
            READ_TIMEOUT,
        ))?)),
        ManagerId::Mas => {
            let out = run(&bin(m)?, &["list"], READ_TIMEOUT);
            // Some versions of mas report an empty list as an error.
            if !out.success && out.tail().contains("No installed apps") {
                return Ok(Vec::new());
            }
            Ok(mas::parse_list(&checked(out)?))
        }
    }
}

/// Every installed package id, including dependencies. Used to tell whether
/// something from a setup file is already present.
pub fn installed_ids(m: ManagerId) -> Result<std::collections::HashSet<String>, String> {
    match m {
        ManagerId::Apt => {
            let dpkg = checked(run(
                &tool("dpkg-query")?,
                &["-W", "-f", "${Package}\t${db:Status-Abbrev}\n"],
                READ_TIMEOUT,
            ))?;
            Ok(text::lines(&dpkg)
                .filter_map(|l| l.split_once('\t'))
                .filter(|(_, status)| status.starts_with("ii"))
                .map(|(name, _)| name.to_string())
                .collect())
        }
        ManagerId::Snap => {
            let out = checked(run(&bin(m)?, &["list"], READ_TIMEOUT))?;
            Ok(text::lines(&out)
                .skip(1)
                .filter_map(|l| l.split_whitespace().next())
                .map(str::to_string)
                .collect())
        }
        ManagerId::Brew => {
            let brew = bin(m)?;
            let out = checked(run(&brew, &["list", "--formula", "-1"], READ_TIMEOUT))?;
            Ok(text::lines(&out).map(|l| l.trim().to_string()).collect())
        }
        ManagerId::BrewCask => {
            let out = checked(run(&bin(m)?, &["list", "--cask", "-1"], READ_TIMEOUT))?;
            Ok(text::lines(&out).map(|l| l.trim().to_string()).collect())
        }
        _ => Ok(installed(m)?
            .into_iter()
            .flat_map(|p| [p.id, p.name])
            .collect()),
    }
}

/// Packages that have an update waiting.
pub fn updates(m: ManagerId) -> Result<Vec<Package>, String> {
    match m {
        ManagerId::Apt => {
            let apt = tool("apt")?;
            let mut list = apt::parse_upgradable(&checked(run(
                &apt,
                &["list", "--upgradable"],
                READ_TIMEOUT,
            ))?);
            if !list.is_empty() {
                // A simulated upgrade (no root needed) tells which of these
                // apt would hold back.
                let sim = run(
                    &tool("apt-get")?,
                    &[
                        "-s",
                        "-o",
                        "Debug::NoLocking=1",
                        "upgrade",
                        "--with-new-pkgs",
                    ],
                    READ_TIMEOUT,
                );
                if sim.success {
                    let held = apt::parse_held(&sim.stdout);
                    for p in &mut list {
                        if let Some(reason) = held.get(&p.id) {
                            p.held = reason.clone();
                        }
                    }
                }
            }
            Ok(list)
        }
        ManagerId::Snap => {
            let out = run(&bin(m)?, &["refresh", "--list"], READ_TIMEOUT);
            // "All snaps up to date." is printed to stderr with exit code 0.
            let mut list = snap::parse_refresh_list(&checked(out)?);
            if !list.is_empty() {
                let current: HashMap<String, String> = installed(m)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| (p.id, p.version))
                    .collect();
                for p in &mut list {
                    p.version = current.get(&p.id).cloned().unwrap_or_default();
                }
            }
            Ok(list)
        }
        ManagerId::Flatpak => {
            let flatpak = bin(m)?;
            let current: HashMap<String, String> = installed(m)
                .unwrap_or_default()
                .into_iter()
                .map(|p| (format!("{}/{}", p.scope, p.id), p.version))
                .collect();
            let mut list = Vec::new();
            for scope in ["system", "user"] {
                let flag = format!("--{scope}");
                let out = run(
                    &flatpak,
                    &[
                        "remote-ls",
                        &flag,
                        "--updates",
                        "--app",
                        "--columns=application,name,version,origin",
                    ],
                    READ_TIMEOUT,
                );
                if out.success {
                    list.extend(flatpak::parse_updates(&out.stdout, scope));
                }
            }
            for p in &mut list {
                p.version = current
                    .get(&format!("{}/{}", p.scope, p.id))
                    .cloned()
                    .unwrap_or_default();
            }
            Ok(list)
        }
        ManagerId::Brew | ManagerId::BrewCask => {
            let flag = if m == ManagerId::Brew {
                "--formula"
            } else {
                "--cask"
            };
            Ok(brew::parse_outdated(&checked(run(
                &bin(m)?,
                &["outdated", flag, "--json=v2"],
                READ_TIMEOUT,
            ))?)
            .into_iter()
            .filter(|p| p.manager == Some(m))
            .collect())
        }
        ManagerId::Mas => Ok(mas::parse_outdated(&checked(run(
            &bin(m)?,
            &["outdated"],
            READ_TIMEOUT,
        ))?)),
    }
}

pub fn search(m: ManagerId, query: &str) -> Result<Vec<Package>, String> {
    let query = query.trim();
    if query.is_empty()
        || query.len() > 100
        || query.starts_with('-')
        || query.contains(['\n', '\r'])
    {
        return Ok(Vec::new());
    }
    let mut list = match m {
        ManagerId::Apt => apt::parse_search(&checked(run(
            &tool("apt-cache")?,
            &["search", "--names-only", query],
            SEARCH_TIMEOUT,
        ))?),
        ManagerId::Snap => {
            let out = run(&bin(m)?, &["find", query], SEARCH_TIMEOUT);
            // `snap find` exits with 0 even when nothing matches; a network
            // failure is the case worth reporting.
            if !out.success && !out.stderr.contains("No matching snaps") {
                return Err(out.tail());
            }
            snap::parse_find(&out.stdout)
        }
        ManagerId::Flatpak => flatpak::parse_search(&checked(run(
            &bin(m)?,
            &[
                "search",
                "--columns=application,name,version,remotes,description",
                query,
            ],
            SEARCH_TIMEOUT,
        ))?),
        ManagerId::Brew | ManagerId::BrewCask => {
            let flag = if m == ManagerId::Brew {
                "--formula"
            } else {
                "--cask"
            };
            let out = run(&bin(m)?, &["search", flag, query], SEARCH_TIMEOUT);
            // Exit code 1 means "nothing found".
            if !out.success && out.code != Some(1) {
                return Err(out.tail());
            }
            brew::parse_search(&out.stdout, m)
        }
        ManagerId::Mas => {
            let out = run(&bin(m)?, &["search", query], SEARCH_TIMEOUT);
            // `mas search` exits with an error when nothing matches.
            if !out.success && !out.stdout.trim().is_empty() {
                return Err(out.tail());
            }
            mas::parse_search(&out.stdout)
        }
    };
    rank(&mut list, query);
    list.truncate(SEARCH_LIMIT);
    Ok(list)
}

/// Exact and prefix matches first.
fn rank(list: &mut [Package], query: &str) {
    let q = query.to_lowercase();
    let score = |p: &Package| {
        let id = p.id.to_lowercase();
        let name = p.name.to_lowercase();
        let last = id.rsplit(['.', '/']).next().unwrap_or("").to_string();
        if id == q || name == q || last == q {
            0
        } else if id.starts_with(&q) || name.starts_with(&q) || last.starts_with(&q) {
            1
        } else if id.contains(&q) || name.contains(&q) {
            2
        } else {
            3
        }
    };
    list.sort_by_key(|p| score(p));
}

// ---------------------------------------------------------------------------
// Changing packages

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Install,
    Remove,
    Update,
}

/// One command to run.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub managers: Vec<ManagerId>,
    pub program: PathBuf,
    pub args: Vec<String>,
    /// Package ids this step acts on (empty for "update everything").
    pub packages: Vec<String>,
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// Plan the commands for installing, removing or updating the given packages.
pub fn plan(action: Action, refs: &[PackageRef]) -> Result<Vec<Step>, String> {
    for r in refs {
        if !valid_package_id(&r.id) {
            return Err(format!("Not a valid package name: {}", r.id));
        }
        if !r.scope.is_empty() && !matches!(r.scope.as_str(), "user" | "system") {
            return Err(format!("Not a valid installation: {}", r.scope));
        }
        if !r.origin.is_empty() && !valid_simple_word(&r.origin) {
            return Err(format!("Not a valid remote: {}", r.origin));
        }
    }

    let mut by_manager: Vec<(ManagerId, Vec<&PackageRef>)> = Vec::new();
    for m in ManagerId::ALL {
        let group: Vec<&PackageRef> = refs.iter().filter(|r| r.manager == m).collect();
        if !group.is_empty() {
            by_manager.push((m, group));
        }
    }

    let mut steps = Vec::new();
    let mut admin: Vec<privileged::AdminRequest> = Vec::new();

    for (m, group) in by_manager {
        let ids: Vec<String> = group.iter().map(|r| r.id.clone()).collect();
        match m {
            ManagerId::Apt => admin.push(privileged::AdminRequest::apt(action, ids)),
            ManagerId::Snap => {
                if action == Action::Install {
                    let (classic, strict): (Vec<&PackageRef>, Vec<&PackageRef>) =
                        group.iter().copied().partition(|r| r.classic);
                    if !strict.is_empty() {
                        admin.push(privileged::AdminRequest::snap(
                            action,
                            false,
                            strict.iter().map(|r| r.id.clone()).collect(),
                        ));
                    }
                    if !classic.is_empty() {
                        admin.push(privileged::AdminRequest::snap(
                            action,
                            true,
                            classic.iter().map(|r| r.id.clone()).collect(),
                        ));
                    }
                } else {
                    admin.push(privileged::AdminRequest::snap(action, false, ids));
                }
            }
            ManagerId::Flatpak => steps.extend(plan_flatpak(action, &group)?),
            ManagerId::Brew | ManagerId::BrewCask => {
                let verb = match action {
                    Action::Install => "install",
                    Action::Remove => "uninstall",
                    Action::Update => "upgrade",
                };
                let kind = if m == ManagerId::Brew {
                    "--formula"
                } else {
                    "--cask"
                };
                let mut args = strings(&[verb, kind]);
                args.extend(ids.iter().cloned());
                steps.push(Step {
                    managers: vec![m],
                    program: bin(m)?,
                    args,
                    packages: ids,
                });
            }
            ManagerId::Mas => steps.extend(plan_mas(action, &ids)?),
        }
    }

    if !admin.is_empty() {
        // apt and snap go first and in a single authentication.
        let mut all = privileged::admin_steps(&admin)?;
        all.extend(steps);
        steps = all;
    }
    Ok(steps)
}

/// Installing and updating App Store apps needs the App Store itself (`mas`
/// asks for an administrator password in a terminal, which a window cannot
/// answer). So Suisha opens the right page of the App Store instead.
pub fn plan_mas(action: Action, ids: &[String]) -> Result<Vec<Step>, String> {
    if ids
        .iter()
        .any(|id| id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()))
    {
        return Err("Not a valid App Store app".into());
    }
    if action == Action::Remove {
        return Err("Remove App Store apps from Finder or Launchpad.".into());
    }
    let open = open_tool()?;
    match action {
        Action::Install => Ok(ids
            .iter()
            .map(|id| Step {
                managers: vec![ManagerId::Mas],
                program: open.clone(),
                args: vec![app_store_page(id)],
                packages: vec![id.clone()],
            })
            .collect()),
        Action::Update => Ok(vec![Step {
            managers: vec![ManagerId::Mas],
            program: open,
            args: vec![APP_STORE_UPDATES.into()],
            packages: ids.to_vec(),
        }]),
        Action::Remove => unreachable!("handled above"),
    }
}

fn plan_flatpak(action: Action, group: &[&PackageRef]) -> Result<Vec<Step>, String> {
    let flatpak = bin(ManagerId::Flatpak)?;
    let remotes: HashMap<String, String> = {
        let out = run(
            &flatpak,
            &["remotes", "--columns=name,options"],
            READ_TIMEOUT,
        );
        flatpak::parse_remotes(&out.stdout).into_iter().collect()
    };
    let mut steps = Vec::new();
    for r in group {
        let mut args = match action {
            Action::Install => strings(&["install", "-y", "--noninteractive"]),
            Action::Remove => strings(&["uninstall", "-y", "--noninteractive"]),
            Action::Update => strings(&["update", "-y", "--noninteractive"]),
        };
        let scope = if !r.scope.is_empty() {
            r.scope.clone()
        } else {
            remotes
                .get(if r.origin.is_empty() {
                    "flathub"
                } else {
                    &r.origin
                })
                .cloned()
                .unwrap_or_else(|| "system".into())
        };
        args.push(format!("--{scope}"));
        if action == Action::Install {
            args.push(if r.origin.is_empty() {
                "flathub".into()
            } else {
                r.origin.clone()
            });
        }
        args.push(r.id.clone());
        steps.push(Step {
            managers: vec![ManagerId::Flatpak],
            program: flatpak.clone(),
            args,
            packages: vec![r.id.clone()],
        });
    }
    Ok(steps)
}

/// Commands that update everything. `include_admin` adds apt and snap.
/// `interactive` says that someone is there: a password can be asked for, and
/// the App Store's updates page is opened when App Store apps have updates (a
/// scheduled run leaves them to the App Store's own setting).
pub fn plan_update_all(
    managers: &[ManagerId],
    include_admin: bool,
    interactive: bool,
) -> Result<Vec<Step>, String> {
    let mut steps = Vec::new();
    let wants = |m: ManagerId| managers.contains(&m) && binary(m).is_some();

    if include_admin && (wants(ManagerId::Apt) || wants(ManagerId::Snap)) {
        steps.push(privileged::update_all_step(interactive)?);
    }
    if wants(ManagerId::Flatpak) {
        steps.push(Step {
            managers: vec![ManagerId::Flatpak],
            program: bin(ManagerId::Flatpak)?,
            args: strings(&["update", "-y", "--noninteractive"]),
            packages: vec![],
        });
    }
    let brew_managers: Vec<ManagerId> = [ManagerId::Brew, ManagerId::BrewCask]
        .into_iter()
        .filter(|m| wants(*m))
        .collect();
    if let Some(&first) = brew_managers.first() {
        let brew = bin(first)?;
        steps.push(Step {
            managers: brew_managers.clone(),
            program: brew.clone(),
            args: strings(&["update"]),
            packages: vec![],
        });
        for m in &brew_managers {
            let kind = if *m == ManagerId::Brew {
                "--formula"
            } else {
                "--cask"
            };
            steps.push(Step {
                managers: vec![*m],
                program: brew.clone(),
                args: strings(&["upgrade", kind]),
                packages: vec![],
            });
        }
    }
    if interactive && wants(ManagerId::Mas) {
        let waiting: Vec<String> = updates(ManagerId::Mas)
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.id)
            .collect();
        if !waiting.is_empty() {
            steps.extend(plan_mas(Action::Update, &waiting)?);
        }
    }
    Ok(steps)
}

/// Fetch fresh package information where that needs no password (`brew update`).
pub fn refresh_steps() -> Vec<Step> {
    match binary(ManagerId::Brew) {
        Some(brew) => vec![Step {
            managers: vec![ManagerId::Brew],
            program: brew,
            args: strings(&["update"]),
            packages: vec![],
        }],
        None => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(m: ManagerId, id: &str) -> PackageRef {
        PackageRef {
            manager: m,
            id: id.into(),
            scope: String::new(),
            origin: String::new(),
            classic: false,
        }
    }

    #[test]
    fn rejects_bad_names() {
        assert!(plan(Action::Install, &[r(ManagerId::Apt, "-y")]).is_err());
        assert!(plan(Action::Install, &[r(ManagerId::Apt, "a b")]).is_err());
        let mut bad_scope = r(ManagerId::Flatpak, "org.x.Y");
        bad_scope.scope = "--system".into();
        assert!(plan(Action::Remove, &[bad_scope]).is_err());
        assert!(plan_mas(Action::Install, &["xcode".into()]).is_err());
    }

    #[test]
    fn app_store_links() {
        assert_eq!(
            app_store_page("497799835"),
            "macappstore://apps.apple.com/app/id497799835"
        );
        // Removing goes through Finder; this is checked before `open` is needed.
        assert!(plan_mas(Action::Remove, &["497799835".into()]).is_err());
    }

    #[test]
    fn mac_only_managers_are_hidden_on_linux() {
        if cfg!(target_os = "linux") {
            assert!(binary(ManagerId::BrewCask).is_none());
            assert!(binary(ManagerId::Mas).is_none());
        } else {
            assert!(binary(ManagerId::Apt).is_none());
            assert!(binary(ManagerId::Snap).is_none());
        }
    }

    #[test]
    fn ranks_exact_first() {
        let mut list = vec![
            Package::new(ManagerId::Apt, "vim-gtk3"),
            Package::new(ManagerId::Apt, "neovim"),
            Package::new(ManagerId::Apt, "vim"),
        ];
        rank(&mut list, "vim");
        let ids: Vec<_> = list.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["vim", "vim-gtk3", "neovim"]);
    }

    #[test]
    fn search_ignores_option_like_queries() {
        assert!(search(ManagerId::Apt, "--help").unwrap().is_empty());
        assert!(search(ManagerId::Apt, "  ").unwrap().is_empty());
    }
}
