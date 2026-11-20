//! Things about the system that are worth a notice in the window: a pending
//! restart, packages that are no longer needed, and a Flatpak without Flathub.

use std::fs;
use std::time::Duration;

use serde::Serialize;

use crate::exec::{find_bin, run};
use crate::managers::Step;
use crate::model::ManagerId;

const REBOOT_FLAG: &str = "/var/run/reboot-required";
const REBOOT_PACKAGES: &str = "/var/run/reboot-required.pkgs";
pub const FLATHUB_URL: &str = "https://dl.flathub.org/repo/flathub.flatpakrepo";

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatus {
    /// Ubuntu asks for a restart to finish installing updates.
    pub restart_required: bool,
    /// The packages that asked for it.
    pub restart_packages: Vec<String>,
    /// Packages installed as dependencies that nothing needs any more.
    pub unneeded: Vec<String>,
    /// Flatpak is installed but the Flathub app catalogue is not set up.
    pub flathub_missing: bool,
}

/// `/var/run/reboot-required.pkgs`: one package per line, with repeats.
pub fn parse_restart_packages(text: &str) -> Vec<String> {
    let mut list: Vec<String> = text
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    list.sort();
    list.dedup();
    list
}

/// `apt-get -s autoremove`: lines like `Remv libfoo1 [1.2-3]`.
pub fn parse_autoremove(simulation: &str) -> Vec<String> {
    simulation
        .lines()
        .filter_map(|l| l.strip_prefix("Remv "))
        .filter_map(|rest| rest.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// `flatpak remotes --columns=name`
pub fn has_flathub(remotes: &str) -> bool {
    remotes.lines().any(|l| l.trim() == "flathub")
}

pub fn status() -> SystemStatus {
    let mut s = SystemStatus {
        restart_required: fs::metadata(REBOOT_FLAG).is_ok(),
        ..Default::default()
    };
    if s.restart_required {
        s.restart_packages =
            parse_restart_packages(&fs::read_to_string(REBOOT_PACKAGES).unwrap_or_default());
    }
    if let Some(apt_get) = find_bin("apt-get") {
        let out = run(
            &apt_get,
            &["-s", "-o", "Debug::NoLocking=1", "autoremove"],
            Duration::from_secs(120),
        );
        if out.success {
            s.unneeded = parse_autoremove(&out.stdout);
        }
    }
    if let Some(flatpak) = find_bin("flatpak") {
        let out = run(
            &flatpak,
            &["remotes", "--columns=name"],
            Duration::from_secs(30),
        );
        s.flathub_missing = out.success && !has_flathub(&out.stdout);
    }
    s
}

/// Add Flathub for this user. Needs no password.
pub fn add_flathub_step() -> Result<Step, String> {
    let flatpak = find_bin("flatpak").ok_or("flatpak was not found")?;
    Ok(Step {
        managers: vec![ManagerId::Flatpak],
        program: flatpak,
        args: [
            "remote-add",
            "--user",
            "--if-not-exists",
            "flathub",
            FLATHUB_URL,
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
        packages: vec!["flathub".into()],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_packages() {
        let list =
            parse_restart_packages("linux-image-6.8.0-151-generic\nlinux-base\nlinux-base\n\n");
        assert_eq!(list, vec!["linux-base", "linux-image-6.8.0-151-generic"]);
    }

    #[test]
    fn autoremove() {
        let sim = "NOTE: This is only a simulation!\n\
            Reading package lists...\n\
            The following packages will be REMOVED:\n\
            \x20 linux-headers-7.0.0-42 linux-tools-7.0.0-42\n\
            0 upgraded, 0 newly installed, 2 to remove and 7 not upgraded.\n\
            Remv linux-headers-7.0.0-42 [7.0.0-42.42]\n\
            Remv linux-tools-7.0.0-42 [7.0.0-42.42]\n";
        assert_eq!(
            parse_autoremove(sim),
            vec!["linux-headers-7.0.0-42", "linux-tools-7.0.0-42"]
        );
        assert!(parse_autoremove("0 upgraded, 0 newly installed, 0 to remove.\n").is_empty());
    }

    #[test]
    fn flathub() {
        assert!(has_flathub("fedora\nflathub\n"));
        assert!(!has_flathub("fedora\n"));
        assert!(!has_flathub(""));
    }
}
