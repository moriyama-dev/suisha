//! Keeping Suisha itself up to date.
//!
//! The Suisha package registers its own apt repository, so Suisha is updated
//! together with everything else. On top of that, this module finds out
//! whether a newer release exists (for the notice in the window) and updates
//! Suisha on its own: right away from a button, or once a day by a timer.

use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use crate::exec::{find_bin, run};
use crate::managers::Step;
use crate::privileged::{self, Helper};

pub const RELEASES_URL: &str = "https://github.com/moriyama-dev/suisha/releases";
const LATEST_API: &str = "https://api.github.com/repos/moriyama-dev/suisha/releases/latest";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfUpdate {
    pub current: String,
    pub latest: String,
    pub available: bool,
    /// "apt" (installed from the Suisha package: can update itself) or
    /// "manual" (run from somewhere else, for example a development build).
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

/// Whether this copy came from the Suisha package and can update itself.
pub fn channel() -> &'static str {
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
        auto_allowed: privileged::allowed(Helper::SelfUpdate),
    })
}

/// The command that updates Suisha now.
pub fn update_steps() -> Result<Vec<Step>, String> {
    Ok(vec![privileged::self_update_step()?])
}

/// The version of the installed Suisha package (not of the running program).
pub fn installed_version() -> String {
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
    fn development_builds_do_not_update_themselves() {
        // Tests never run from /usr/bin/suisha.
        assert_eq!(channel(), "manual");
    }
}
