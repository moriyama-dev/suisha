//! Commands that need administrator rights on Linux (apt and snap).
//!
//! The `suisha-cli` package installs small root helpers in `/usr/lib/suisha`
//! together with a polkit policy. The window asks for the password through
//! polkit (`pkexec`), a terminal through `sudo`, and root runs the helpers as
//! they are. When the helpers are missing (for example while developing), the
//! package manager itself is run through `pkexec` or `sudo`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::exec::{find_bin, run};
use crate::managers::{Action, Step};
use crate::model::ManagerId;

pub const HELPER_DIR: &str = "/usr/lib/suisha";
const ACCESS_HELPER: &str = "suisha-auto-update-access";
const SUDOERS_DIR: &str = "/etc/sudoers.d";

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

    fn program(self) -> String {
        format!("suisha-{}", self.name())
    }
}

/// How a command gets administrator rights.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Elevation {
    /// Already root: the command runs as it is.
    Direct,
    /// polkit: a password dialog, or a rule that allows the helper.
    Pkexec(PathBuf),
    /// sudo, which asks for the password in the terminal.
    Sudo(PathBuf),
    /// sudo that fails instead of asking. A timer has nobody to answer.
    SudoNoPrompt(PathBuf),
}

impl Elevation {
    /// The program and arguments that run `command` with administrator rights.
    pub fn wrap(&self, mut command: Vec<String>) -> (PathBuf, Vec<String>) {
        match self {
            Elevation::Direct => {
                let program = PathBuf::from(command.remove(0));
                (program, command)
            }
            Elevation::Pkexec(pkexec) => (pkexec.clone(), command),
            Elevation::Sudo(sudo) => (sudo.clone(), command),
            Elevation::SudoNoPrompt(sudo) => {
                command.insert(0, "-n".into());
                (sudo.clone(), command)
            }
        }
    }

    /// Whether the password is typed into the terminal Suisha runs in.
    pub fn asks_in_terminal(&self) -> bool {
        matches!(self, Elevation::Sudo(_))
    }
}

/// Set by the command line: passwords are asked for in the terminal.
static TERMINAL: AtomicBool = AtomicBool::new(false);

/// Ask for passwords with `sudo` in the terminal instead of a polkit dialog.
pub fn use_terminal() {
    TERMINAL.store(true, Ordering::SeqCst);
}

pub fn is_root() -> bool {
    rustix::process::geteuid().is_root()
}

/// How to get administrator rights while a person is there to type a password.
pub fn interactive() -> Result<Elevation, String> {
    if is_root() {
        return Ok(Elevation::Direct);
    }
    let sudo = || find_bin("sudo").map(Elevation::Sudo);
    let pkexec = || find_bin("pkexec").map(Elevation::Pkexec);
    if TERMINAL.load(Ordering::SeqCst) {
        sudo().or_else(pkexec).ok_or_else(|| {
            "sudo was not found. Run Suisha as root, or install it with: apt install sudo"
                .to_string()
        })
    } else {
        pkexec().ok_or_else(|| {
            "pkexec was not found. Install it with: sudo apt install pkexec".to_string()
        })
    }
}

fn polkit_allows(which: Helper) -> bool {
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

/// The sudoers file `suisha-auto-update-access enable` writes for a user.
pub fn sudoers_file(which: Helper, uid: u32) -> PathBuf {
    Path::new(SUDOERS_DIR).join(format!("suisha-{}-{uid}", which.name()))
}

fn sudo_allows(which: Helper) -> bool {
    sudoers_file(which, rustix::process::getuid().as_raw()).exists()
}

/// How to run a helper when nobody is there to type a password, if this
/// user may do that: root always, others after they allowed it once.
pub fn unattended(which: Helper) -> Option<Elevation> {
    if is_root() {
        return Some(Elevation::Direct);
    }
    if polkit_allows(which)
        && let Some(pkexec) = find_bin("pkexec")
    {
        return Some(Elevation::Pkexec(pkexec));
    }
    if sudo_allows(which) {
        return find_bin("sudo").map(Elevation::SudoNoPrompt);
    }
    None
}

fn helper(name: &str) -> Option<PathBuf> {
    let path = Path::new(HELPER_DIR).join(name);
    path.is_file().then_some(path)
}

pub fn helpers_installed() -> bool {
    helper("suisha-root").is_some()
        && helper("suisha-auto-update").is_some()
        && helper("suisha-self-update").is_some()
        && helper(ACCESS_HELPER).is_some()
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

fn step(
    elevation: &Elevation,
    command: Vec<String>,
    managers: Vec<ManagerId>,
    packages: Vec<String>,
) -> Step {
    let (program, args) = elevation.wrap(command);
    Step {
        managers,
        program,
        args,
        packages,
    }
}

/// With the helper: one `suisha-root ...` call, so one password prompt.
pub fn helper_step(helper: PathBuf, elevation: &Elevation, requests: &[AdminRequest]) -> Step {
    let mut command = vec![helper.to_string_lossy().into_owned()];
    let mut managers = Vec::new();
    let mut packages = Vec::new();
    for req in requests {
        command.push(req.mode.to_string());
        command.extend(req.packages.iter().cloned());
        packages.extend(req.packages.iter().cloned());
        if !managers.contains(&req.manager) {
            managers.push(req.manager);
        }
    }
    step(elevation, command, managers, packages)
}

/// Without the helper: call apt-get / snap itself, one call each.
pub fn direct_steps(elevation: &Elevation, requests: &[AdminRequest]) -> Vec<Step> {
    requests
        .iter()
        .map(|req| {
            let mut command: Vec<String> = match req.mode {
                "--apt-install" => vec!["install".into()],
                "--apt-remove" => vec!["remove".into()],
                "--apt-upgrade" => vec!["install".into(), "--only-upgrade".into()],
                "--snap-install" => vec!["install".into()],
                "--snap-install-classic" => vec!["install".into(), "--classic".into()],
                "--snap-remove" => vec!["remove".into()],
                _ => vec!["refresh".into()],
            };
            if req.manager == ManagerId::Apt {
                command.splice(
                    0..0,
                    [
                        "env".to_string(),
                        "DEBIAN_FRONTEND=noninteractive".into(),
                        "apt-get".into(),
                        "-y".into(),
                    ],
                );
                command.push("--".into());
            } else {
                command.insert(0, "snap".into());
            }
            // `snap install --classic` takes a single snap.
            command.extend(req.packages.iter().cloned());
            step(elevation, command, vec![req.manager], req.packages.clone())
        })
        .collect()
}

pub fn admin_steps(requests: &[AdminRequest]) -> Result<Vec<Step>, String> {
    let elevation = interactive()?;
    match helper("suisha-root") {
        Some(h) => Ok(vec![helper_step(h, &elevation, requests)]),
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
            Ok(direct_steps(&elevation, &expanded))
        }
    }
}

/// The elevation for one of the two password-free helpers: asking for a
/// password when someone is there, and only what was allowed otherwise.
fn elevation_for(which: Helper, attended: bool) -> Result<Elevation, String> {
    if attended {
        interactive()
    } else {
        unattended(which).ok_or_else(|| {
            format!(
                "Running {} without a password has not been allowed.",
                which.program()
            )
        })
    }
}

/// Update every apt and snap package. `attended` says whether someone can
/// type a password.
pub fn update_all_step(attended: bool) -> Result<Step, String> {
    let managers: Vec<ManagerId> = [ManagerId::Apt, ManagerId::Snap]
        .into_iter()
        .filter(|m| crate::managers::binary(*m).is_some())
        .collect();
    let h = helper("suisha-auto-update")
        .ok_or("Updating apt and snap needs the suisha-cli package (its helper was not found).")?;
    let elevation = elevation_for(Helper::AutoUpdate, attended)?;
    Ok(step(
        &elevation,
        vec![h.to_string_lossy().into_owned()],
        managers,
        vec![],
    ))
}

/// Remove packages that were installed as dependencies and are no longer needed.
pub fn autoremove_step() -> Result<Step, String> {
    let elevation = interactive()?;
    let command = match helper("suisha-root") {
        Some(h) => vec![h.to_string_lossy().into_owned(), "--apt-autoremove".into()],
        None => vec![
            "env".into(),
            "DEBIAN_FRONTEND=noninteractive".into(),
            "apt-get".into(),
            "-y".into(),
            "autoremove".into(),
        ],
    };
    Ok(step(&elevation, command, vec![ManagerId::Apt], vec![]))
}

/// Update Suisha itself from its apt repository.
pub fn self_update_step(attended: bool) -> Result<Step, String> {
    let h = helper("suisha-self-update")
        .ok_or("Updating Suisha needs the suisha-cli package (its helper was not found).")?;
    let elevation = elevation_for(Helper::SelfUpdate, attended)?;
    Ok(step(
        &elevation,
        vec![h.to_string_lossy().into_owned()],
        vec![ManagerId::Apt],
        vec!["suisha".into()],
    ))
}

/// Whether this user may already run the helper without a password.
pub fn allowed(which: Helper) -> bool {
    unattended(which).is_some()
}

pub fn auto_update_allowed() -> bool {
    allowed(Helper::AutoUpdate)
}

fn run_access_helper(args: &[&str]) -> Result<(), String> {
    let helper = helper(ACCESS_HELPER)
        .ok_or("The suisha-cli package is needed for this (its helper was not found).")?;
    let elevation = interactive()?;
    let mut command = vec![helper.to_string_lossy().into_owned()];
    command.extend(args.iter().map(|a| a.to_string()));
    let (program, args) = elevation.wrap(command);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = run(&program, &args, Duration::from_secs(600));
    match (&elevation, out.code) {
        (_, Some(0)) => Ok(()),
        // 126: the dialog was dismissed; 127: not authorized.
        (Elevation::Pkexec(_), Some(126) | Some(127)) => Err("cancelled".into()),
        _ => Err(out.tail()),
    }
}

/// Grant or take back running a helper without a password. Asks for the
/// password: a dialog from the window, `sudo` in a terminal.
pub fn set_access(which: Helper, enable: bool) -> Result<(), String> {
    if is_root() {
        // Root needs no permission, and has none to take back.
        return Ok(());
    }
    let arg = if enable { "enable" } else { "disable" };
    run_access_helper(&[arg, which.name()])
}

pub fn set_auto_update_access(enable: bool) -> Result<(), String> {
    set_access(Helper::AutoUpdate, enable)
}

/// Keep this user's timers running while nobody is logged in (`loginctl
/// enable-linger`). On a server that is the normal state.
pub fn enable_linger() -> Result<(), String> {
    if is_root() {
        let loginctl = find_bin("loginctl").ok_or("loginctl was not found")?;
        let out = run(
            &loginctl,
            &["enable-linger", "root"],
            Duration::from_secs(30),
        );
        return if out.success { Ok(()) } else { Err(out.tail()) };
    }
    run_access_helper(&["linger"])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkexec() -> Elevation {
        Elevation::Pkexec("/usr/bin/pkexec".into())
    }

    #[test]
    fn one_helper_call_for_everything() {
        let reqs = vec![
            AdminRequest::apt(Action::Install, vec!["git".into(), "htop".into()]),
            AdminRequest::snap(Action::Install, true, vec!["code".into()]),
        ];
        let step = helper_step("/usr/lib/suisha/suisha-root".into(), &pkexec(), &reqs);
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
        let steps = direct_steps(&pkexec(), &reqs);
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

    #[test]
    fn each_way_of_becoming_root() {
        let command = || vec!["/usr/lib/suisha/suisha-auto-update".to_string()];
        let sudo = PathBuf::from("/usr/bin/sudo");

        let (program, args) = Elevation::Direct.wrap(command());
        assert_eq!(program, PathBuf::from("/usr/lib/suisha/suisha-auto-update"));
        assert!(args.is_empty());

        let (program, args) = Elevation::Sudo(sudo.clone()).wrap(command());
        assert_eq!(program, sudo);
        assert_eq!(args, command());

        // A timer must never wait for a password.
        let (program, args) = Elevation::SudoNoPrompt(sudo.clone()).wrap(command());
        assert_eq!(program, sudo);
        assert_eq!(args, vec!["-n", "/usr/lib/suisha/suisha-auto-update"]);
    }

    #[test]
    fn root_runs_the_package_manager_itself() {
        let reqs = vec![AdminRequest::snap(
            Action::Update,
            false,
            vec!["lxd".into()],
        )];
        let steps = direct_steps(&Elevation::Direct, &reqs);
        assert_eq!(steps[0].program, PathBuf::from("snap"));
        assert_eq!(steps[0].args, vec!["refresh", "lxd"]);
    }

    #[test]
    fn sudoers_file_has_no_dot_in_its_name() {
        // sudo skips files in /etc/sudoers.d whose name contains a dot, so
        // the file is named after the user id, not the user name.
        let path = sudoers_file(Helper::AutoUpdate, 1000);
        assert_eq!(
            path,
            PathBuf::from("/etc/sudoers.d/suisha-auto-update-1000")
        );
        assert!(!path.file_name().unwrap().to_string_lossy().contains('.'));
    }
}
