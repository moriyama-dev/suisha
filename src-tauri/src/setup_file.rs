//! The setup file: a list of installed apps written on one computer and read
//! on another to install the same set.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::managers;
use crate::model::{ManagerId, Package, PackageRef, valid_package_id};

pub const FORMAT: &str = "suisha-setup";
pub const VERSION: u32 = 1;
const MAX_SIZE: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupFile {
    pub format: String,
    pub version: u32,
    /// Always "linux". Lists from another system are not opened.
    pub os: String,
    pub exported_at: String,
    pub packages: Vec<SetupEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupEntry {
    pub manager: ManagerId,
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub scope: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub origin: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub classic: bool,
}

impl From<&Package> for SetupEntry {
    fn from(p: &Package) -> Self {
        SetupEntry {
            manager: p.manager.unwrap_or(ManagerId::Apt),
            id: p.id.clone(),
            name: if p.name == p.id {
                String::new()
            } else {
                p.name.clone()
            },
            scope: p.scope.clone(),
            origin: p.origin.clone(),
            classic: p.classic,
        }
    }
}

pub fn current_os() -> &'static str {
    "linux"
}

pub fn build(packages: &[Package]) -> SetupFile {
    SetupFile {
        format: FORMAT.into(),
        version: VERSION,
        os: current_os().into(),
        exported_at: crate::store::now(),
        packages: packages.iter().map(SetupEntry::from).collect(),
    }
}

pub fn write(path: &Path, file: &SetupFile) -> Result<(), String> {
    let text = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    fs::write(path, text + "\n").map_err(|e| e.to_string())
}

/// Read and check a setup file. Error strings are message keys the window
/// translates.
pub fn read(path: &Path) -> Result<SetupFile, String> {
    let size = fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > MAX_SIZE {
        return Err("setup.tooLarge".into());
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse(&text)
}

pub fn parse(text: &str) -> Result<SetupFile, String> {
    let file: SetupFile =
        serde_json::from_str(text).map_err(|_| "setup.notASetupFile".to_string())?;
    if file.format != FORMAT {
        return Err("setup.notASetupFile".into());
    }
    if file.version > VERSION {
        return Err("setup.newerVersion".into());
    }
    if file.os != current_os() {
        return Err(format!("setup.otherOs:{}", file.os));
    }
    Ok(file)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewEntry {
    #[serde(flatten)]
    pub package: PackageRef,
    pub name: String,
    /// "missing", "installed", "unavailable" (manager not present) or "invalid".
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub exported_at: String,
    pub entries: Vec<PreviewEntry>,
}

/// Compare a setup file with what is installed here.
pub fn preview(file: &SetupFile) -> Preview {
    let available = managers::available();
    let mut installed: HashSet<(ManagerId, String)> = HashSet::new();
    let used: HashSet<ManagerId> = file.packages.iter().map(|e| e.manager).collect();
    for m in &available {
        if used.contains(m) {
            for id in managers::installed_ids(*m).unwrap_or_default() {
                // Tapped formulae are listed by their short name.
                let short = id.rsplit('/').next().unwrap_or(&id).to_string();
                installed.insert((*m, short));
                installed.insert((*m, id));
            }
        }
    }
    let entries = file
        .packages
        .iter()
        .map(|e| {
            let status = if !valid_package_id(&e.id) {
                "invalid"
            } else if !available.contains(&e.manager) {
                "unavailable"
            } else if installed.contains(&(e.manager, e.id.clone()))
                || installed.contains(&(
                    e.manager,
                    e.id.rsplit('/').next().unwrap_or(&e.id).to_string(),
                ))
            {
                "installed"
            } else {
                "missing"
            };
            PreviewEntry {
                package: PackageRef {
                    manager: e.manager,
                    id: e.id.clone(),
                    scope: e.scope.clone(),
                    origin: e.origin.clone(),
                    classic: e.classic,
                },
                name: if e.name.is_empty() {
                    e.id.clone()
                } else {
                    e.name.clone()
                },
                status: status.into(),
            }
        })
        .collect();
    Preview {
        exported_at: file.exported_at.clone(),
        entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut snap = Package::new(ManagerId::Snap, "code");
        snap.classic = true;
        let file = build(&[Package::new(ManagerId::Apt, "git"), snap]);
        let text = serde_json::to_string(&file).unwrap();
        assert!(!text.contains("\"scope\""));
        let back = parse(&text).unwrap();
        assert_eq!(back.packages.len(), 2);
        assert!(back.packages[1].classic);
    }

    #[test]
    fn rejects_other_files() {
        assert_eq!(parse("{}").unwrap_err(), "setup.notASetupFile");
        assert_eq!(parse("not json").unwrap_err(), "setup.notASetupFile");
        let other = "macos";
        let text = format!(
            r#"{{"format":"suisha-setup","version":1,"os":"{other}","exportedAt":"x","packages":[]}}"#
        );
        assert_eq!(parse(&text).unwrap_err(), format!("setup.otherOs:{other}"));
        let newer = format!(
            r#"{{"format":"suisha-setup","version":99,"os":"{}","exportedAt":"x","packages":[]}}"#,
            current_os()
        );
        assert_eq!(parse(&newer).unwrap_err(), "setup.newerVersion");
    }
}
