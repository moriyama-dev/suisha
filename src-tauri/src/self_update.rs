//! Keeping Suisha itself up to date.
//!
//! On Linux the Suisha package registers its own apt repository, and on macOS
//! Suisha is a Homebrew cask, so Suisha is updated together with everything
//! else. On top of that, this module finds out whether a newer release exists
//! (for the notice in the window) and updates Suisha on its own: right away
//! from a button, or once a day by a timer.

use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use crate::exec::{find_bin, run};
use crate::managers::Step;
use crate::model::ManagerId;
use crate::privileged::{self, Helper};

pub const RELEASES_URL: &str = "https://github.com/moriyama-dev/suisha/releases";
const LATEST_API: &str = "https://api.github.com/repos/moriyama-dev/suisha/releases/latest";
/// The cask in Suisha's own Homebrew tap.
pub const CASK: &str = "moriyama-dev/suisha/suisha";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfUpdate {
    pub current: String,
    pub latest: String,
    pub available: bool,
    /// "apt" (installed from the Suisha package: can update itself), "brew"
    /// (installed with Homebrew on macOS: can update itself) or "manual" (a
    /// downloaded disk image or a development build).
    pub channel: String,
    pub releases_url: String,
    /// The daily update may run without a password.
    pub auto_allowed: bool,
}

/// Compare "1.2.3" style versions. Anything that is not a number counts as 0.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split(['.', '-', '+'])
            .take(3)
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    parse(candidate) > parse(current)
}

/// Ask GitHub for the newest release. Only a plain request for the public
/// release information is sent.
fn latest_release() -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into();
    let mut response = agent
        .get(LATEST_API)
        .header(
            "User-Agent",
            &format!("Suisha/{}", env!("CARGO_PKG_VERSION")),
        )
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    json.get("tag_name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim_start_matches('v').to_string())
        .ok_or_else(|| "No release found".into())
}

fn brew_cask_installed() -> bool {
    find_bin("brew")
        .is_some_and(|brew| run(&brew, &["list", "--cask", CASK], Duration::from_secs(60)).success)
}

/// Whether this copy came from the Suisha package (Linux) or the Homebrew
/// cask (macOS) and can update itself.
pub fn channel() -> &'static str {
    if cfg!(target_os = "macos") {
        let in_apps =
            std::env::current_exe().is_ok_and(|exe| exe.starts_with("/Applications/Suisha.app"));
        return if in_apps && brew_cask_installed() {
            "brew"
        } else {
            "manual"
        };
    }
    let packaged = std::env::current_exe().is_ok_and(|exe| exe == Path::new("/usr/bin/suisha"));
    if packaged && privileged::helpers_installed() {
        "apt"
    } else {
        "manual"
    }
}

pub fn check() -> Result<SelfUpdate, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let latest = latest_release()?;
    Ok(SelfUpdate {
        available: is_newer(&latest, &current),
        current,
        latest,
        channel: channel().into(),
        releases_url: RELEASES_URL.into(),
        // Homebrew needs no password for Suisha.
        auto_allowed: cfg!(target_os = "macos") || privileged::allowed(Helper::SelfUpdate),
    })
}

/// The package manager that updates Suisha.
pub fn manager() -> ManagerId {
    if cfg!(target_os = "macos") {
        ManagerId::BrewCask
    } else {
        ManagerId::Apt
    }
}

/// Whether "update Suisha automatically" can be offered.
pub fn auto_supported() -> bool {
    if cfg!(target_os = "macos") {
        channel() == "brew"
    } else {
        privileged::helpers_installed()
    }
}

/// The command that updates Suisha now.
pub fn update_steps() -> Result<Vec<Step>, String> {
    if cfg!(target_os = "macos") {
        let brew = find_bin("brew").ok_or("brew was not found")?;
        return Ok(vec![
            Step {
                managers: vec![ManagerId::BrewCask],
                program: brew.clone(),
                args: vec!["update".into()],
                packages: vec![],
            },
            Step {
                managers: vec![ManagerId::BrewCask],
                program: brew,
                args: vec!["upgrade".into(), "--cask".into(), CASK.into()],
                packages: vec!["suisha".into()],
            },
        ]);
    }
    Ok(vec![privileged::self_update_step()?])
}

/// `brew list --cask --versions <cask>`: `suisha 0.5.0`
pub fn parse_cask_version(out: &str) -> String {
    out.lines()
        .next()
        .and_then(|l| l.split_whitespace().last())
        .filter(|v| v.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .unwrap_or_default()
        .to_string()
}

/// The version of the installed Suisha package (not of the running program).
pub fn installed_version() -> String {
    if cfg!(target_os = "macos") {
        return find_bin("brew")
            .map(|brew| {
                parse_cask_version(
                    &run(
                        &brew,
                        &["list", "--cask", "--versions", CASK],
                        Duration::from_secs(60),
                    )
                    .stdout,
                )
            })
            .unwrap_or_default();
    }
    find_bin("dpkg-query")
        .map(|dpkg_query| {
            run(
                &dpkg_query,
                &["-W", "-f", "${Version}", "suisha"],
                Duration::from_secs(20),
            )
            .stdout
            .trim()
            .to_string()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.3.0", "0.2.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(is_newer("v0.2.1", "0.2.0"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.1.9", "0.2.0"));
    }

    #[test]
    fn cask_versions() {
        assert_eq!(parse_cask_version("suisha 0.5.0\n"), "0.5.0");
        assert_eq!(parse_cask_version(""), "");
        assert_eq!(
            parse_cask_version("Error: Cask 'suisha' is not installed.\n"),
            ""
        );
    }

    #[test]
    fn development_builds_do_not_update_themselves() {
        // Tests never run from /usr/bin/suisha.
        assert_eq!(channel(), "manual");
    }
}
