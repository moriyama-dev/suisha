//! Commands that need administrator rights on Linux (apt and snap).
//!
//! The `.deb` installs small root helpers in `/usr/lib/suisha` together with a
//! polkit policy. When they are missing (for example while developing), the
//! package manager is run through `pkexec` directly.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::exec::{find_bin, run};
use crate::managers::{Action, Step};
use crate::model::ManagerId;

pub const HELPER_DIR: &str = "/usr/lib/suisha";
/// The two helpers a user can allow to run without a password.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Helper {
    /// Updates every apt and snap package.
    AutoUpdate,
    /// Updates Suisha itself.
    SelfUpdate,
}

impl Helper {
    pub fn name(self) -> &'static str {
        match self {
            Helper::AutoUpdate => "auto-update",
            Helper::SelfUpdate => "self-update",
        }
    }

    fn action(self) -> String {
        format!("dev.moriyama.suisha.{}", self.name())
    }
}

fn helper(name: &str) -> Option<PathBuf> {
    let path = Path::new(HELPER_DIR).join(name);
    path.is_file().then_some(path)
}

pub fn helpers_installed() -> bool {
    helper("suisha-root").is_some()
        && helper("suisha-auto-update").is_some()
        && helper("suisha-self-update").is_some()
        && helper("suisha-auto-update-access").is_some()
}

fn pkexec() -> Result<PathBuf, String> {
    find_bin("pkexec")
        .ok_or_else(|| "pkexec was not found. Install it with: sudo apt install pkexec".to_string())
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdminRequest {
    manager: ManagerId,
    mode: &'static str,
    packages: Vec<String>,
}

impl AdminRequest {
    pub fn apt(action: Action, packages: Vec<String>) -> Self {
        let mode = match action {
            Action::Install => "--apt-install",
            Action::Remove => "--apt-remove",
            Action::Update => "--apt-upgrade",
        };
        AdminRequest {
            manager: ManagerId::Apt,
            mode,
            packages,
        }
    }

    pub fn snap(action: Action, classic: bool, packages: Vec<String>) -> Self {
        let mode = match (action, classic) {
            (Action::Install, true) => "--snap-install-classic",
            (Action::Install, false) => "--snap-install",
            (Action::Remove, _) => "--snap-remove",
            (Action::Update, _) => "--snap-refresh",
        };
        AdminRequest {
            manager: ManagerId::Snap,
            mode,
            packages,
        }
    }
}

/// With the helper: one `pkexec suisha-root ...` call, so one password prompt.
pub fn helper_step(helper: PathBuf, pkexec: PathBuf, requests: &[AdminRequest]) -> Step {
    let mut args = vec![helper.to_string_lossy().into_owned()];
    let mut managers = Vec::new();
    let mut packages = Vec::new();
    for req in requests {
        args.push(req.mode.to_string());
        args.extend(req.packages.iter().cloned());
        packages.extend(req.packages.iter().cloned());
        if !managers.contains(&req.manager) {
            managers.push(req.manager);
        }
    }
    Step {
        managers,
        program: pkexec,
        args,
        packages,
    }
}

/// Without the helper: call apt-get / snap through pkexec, one call each.
pub fn direct_steps(pkexec: &Path, requests: &[AdminRequest]) -> Vec<Step> {
    requests
        .iter()
        .map(|req| {
            let mut args: Vec<String> = match req.mode {
                "--apt-install" => vec!["install".into()],
                "--apt-remove" => vec!["remove".into()],
                "--apt-upgrade" => vec!["install".into(), "--only-upgrade".into()],
                "--snap-install" => vec!["install".into()],
                "--snap-install-classic" => vec!["install".into(), "--classic".into()],
                "--snap-remove" => vec!["remove".into()],
                _ => vec!["refresh".into()],
            };
            let program = if req.manager == ManagerId::Apt {
                args.splice(
                    0..0,
                    [
                        "env".to_string(),
                        "DEBIAN_FRONTEND=noninteractive".into(),
                        "apt-get".into(),
                        "-y".into(),
                    ],
                );
                args.push("--".into());
                pkexec.to_path_buf()
            } else {
                args.insert(0, "snap".into());
                pkexec.to_path_buf()
            };
            // `snap install --classic` takes a single snap.
            args.extend(req.packages.iter().cloned());
            Step {
                managers: vec![req.manager],
                program,
                args,
                packages: req.packages.clone(),
            }
        })
        .collect()
}

pub fn admin_steps(requests: &[AdminRequest]) -> Result<Vec<Step>, String> {
    let pkexec = pkexec()?;
    match helper("suisha-root") {
        Some(h) => Ok(vec![helper_step(h, pkexec, requests)]),
        None => {
            // Split classic snaps into one call per snap.
            let mut expanded = Vec::new();
            for req in requests {
                if req.mode == "--snap-install-classic" {
                    for p in &req.packages {
                        expanded.push(AdminRequest {
                            packages: vec![p.clone()],
                            ..req.clone()
                        });
                    }
                } else {
                    expanded.push(req.clone());
                }
            }
            Ok(direct_steps(&pkexec, &expanded))
        }
    }
}

/// Update every apt and snap package.
pub fn update_all_step() -> Result<Step, String> {
    let pkexec = pkexec()?;
    let managers: Vec<ManagerId> = [ManagerId::Apt, ManagerId::Snap]
        .into_iter()
        .filter(|m| crate::managers::binary(*m).is_some())
        .collect();
    match helper("suisha-auto-update") {
        Some(h) => Ok(Step {
            managers,
            program: pkexec,
            args: vec![h.to_string_lossy().into_owned()],
            packages: vec![],
        }),
        None => Err(
            "Updating apt and snap needs the Suisha .deb package (its helper was not found)."
                .into(),
        ),
    }
}

/// Remove packages that were installed as dependencies and are no longer needed.
pub fn autoremove_step() -> Result<Step, String> {
    let pkexec = pkexec()?;
    let args = match helper("suisha-root") {
        Some(h) => vec![h.to_string_lossy().into_owned(), "--apt-autoremove".into()],
        None => vec![
            "env".into(),
            "DEBIAN_FRONTEND=noninteractive".into(),
            "apt-get".into(),
            "-y".into(),
            "autoremove".into(),
        ],
    };
    Ok(Step {
        managers: vec![ManagerId::Apt],
        program: pkexec,
        args,
        packages: vec![],
    })
}

/// Update Suisha itself from its apt repository.
pub fn self_update_step() -> Result<Step, String> {
    let pkexec = pkexec()?;
    let h = helper("suisha-self-update")
        .ok_or("Updating Suisha needs the Suisha .deb package (its helper was not found).")?;
    Ok(Step {
        managers: vec![ManagerId::Apt],
        program: pkexec,
        args: vec![h.to_string_lossy().into_owned()],
        packages: vec!["suisha".into()],
    })
}

/// Whether this user may already run the helper without a password.
pub fn allowed(which: Helper) -> bool {
    let Some(pkcheck) = find_bin("pkcheck") else {
        return false;
    };
    let pid = std::process::id().to_string();
    run(
        &pkcheck,
        &["--action-id", &which.action(), "--process", &pid],
        Duration::from_secs(20),
    )
    .success
}

pub fn auto_update_allowed() -> bool {
    allowed(Helper::AutoUpdate)
}

/// Grant or take back running a helper without a password. Shows the system
/// password dialog.
pub fn set_access(which: Helper, enable: bool) -> Result<(), String> {
    let helper = helper("suisha-auto-update-access")
        .ok_or("The Suisha .deb package is needed for this (its helper was not found).")?;
    let pkexec = pkexec()?;
    let arg = if enable { "enable" } else { "disable" };
    let out = run(
        &pkexec,
        &[&helper.to_string_lossy(), arg, which.name()],
        Duration::from_secs(600),
    );
    match out.code {
        Some(0) => Ok(()),
        // 126: the dialog was dismissed; 127: not authorized.
        Some(126) | Some(127) => Err("cancelled".into()),
        _ => Err(out.tail()),
    }
}

pub fn set_auto_update_access(enable: bool) -> Result<(), String> {
    set_access(Helper::AutoUpdate, enable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_helper_call_for_everything() {
        let reqs = vec![
            AdminRequest::apt(Action::Install, vec!["git".into(), "htop".into()]),
            AdminRequest::snap(Action::Install, true, vec!["code".into()]),
        ];
        let step = helper_step(
            "/usr/lib/suisha/suisha-root".into(),
            "/usr/bin/pkexec".into(),
            &reqs,
        );
        assert_eq!(step.program, PathBuf::from("/usr/bin/pkexec"));
        assert_eq!(
            step.args,
            vec![
                "/usr/lib/suisha/suisha-root",
                "--apt-install",
                "git",
                "htop",
                "--snap-install-classic",
                "code"
            ]
        );
        assert_eq!(step.managers, vec![ManagerId::Apt, ManagerId::Snap]);
    }

    #[test]
    fn direct_calls_without_helper() {
        let reqs = vec![
            AdminRequest::apt(Action::Update, vec!["git".into()]),
            AdminRequest::snap(Action::Remove, false, vec!["vlc".into()]),
        ];
        let steps = direct_steps(Path::new("/usr/bin/pkexec"), &reqs);
        assert_eq!(
            steps[0].args,
            vec![
                "env",
                "DEBIAN_FRONTEND=noninteractive",
                "apt-get",
                "-y",
                "install",
                "--only-upgrade",
                "--",
                "git"
            ]
        );
        assert_eq!(steps[1].args, vec!["snap", "remove", "vlc"]);
    }
}
