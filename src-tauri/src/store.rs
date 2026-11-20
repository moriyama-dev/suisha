//! Settings and the update history, kept as small files in the user's
//! configuration and data folders.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::ManagerId;

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("suisha")
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("suisha")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Frequency {
    /// Every `every_hours` hours.
    Hours,
    Daily,
    Weekly,
}

/// The allowed intervals for [`Frequency::Hours`].
pub const HOUR_STEPS: [u8; 4] = [1, 3, 6, 12];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AutoUpdate {
    pub enabled: bool,
    pub frequency: Frequency,
    /// Used when `frequency` is hours.
    pub every_hours: u8,
    /// 0 = Sunday ... 6 = Saturday. Used when `frequency` is weekly.
    pub weekday: u8,
    pub hour: u8,
    pub minute: u8,
    /// Managers to update. Empty means all that are available.
    pub managers: Vec<ManagerId>,
    pub notify: bool,
}

impl Default for AutoUpdate {
    fn default() -> Self {
        AutoUpdate {
            enabled: false,
            frequency: Frequency::Daily,
            every_hours: 6,
            weekday: 1,
            hour: 12,
            minute: 0,
            managers: Vec::new(),
            notify: true,
        }
    }
}

impl AutoUpdate {
    pub fn is_valid(&self) -> bool {
        self.weekday <= 6
            && self.hour <= 23
            && self.minute <= 59
            && (self.frequency != Frequency::Hours || HOUR_STEPS.contains(&self.every_hours))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub auto_update: AutoUpdate,
    /// Update Suisha itself once a day, without asking.
    pub self_update_auto: bool,
    /// The language the window last used ("en" or "ja"), so that background
    /// notifications match it.
    pub language: String,
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn load_settings() -> Settings {
    fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tmp = dir.join("settings.json.tmp");
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, settings_path()).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    /// RFC 3339, UTC.
    pub time: String,
    /// "manual" or "scheduled".
    pub trigger: String,
    /// "install", "remove", "update", "update-all" or "refresh".
    pub action: String,
    pub managers: Vec<ManagerId>,
    pub packages: Vec<String>,
    pub success: bool,
    #[serde(default)]
    pub message: String,
}

const HISTORY_KEEP: usize = 1000;

fn history_path() -> PathBuf {
    data_dir().join("history.jsonl")
}

pub fn append_history(entries: &[HistoryEntry]) {
    if entries.is_empty() {
        return;
    }
    let dir = data_dir();
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path())
    else {
        return;
    };
    for e in entries {
        if let Ok(line) = serde_json::to_string(e) {
            let _ = writeln!(file, "{line}");
        }
    }
    drop(file);
    trim_history();
}

fn trim_history() {
    let Ok(text) = fs::read_to_string(history_path()) else {
        return;
    };
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= HISTORY_KEEP + 200 {
        return;
    }
    let kept = lines[lines.len() - HISTORY_KEEP..].join("\n") + "\n";
    let tmp = data_dir().join("history.jsonl.tmp");
    if fs::write(&tmp, kept).is_ok() {
        let _ = fs::rename(tmp, history_path());
    }
}

/// Newest first.
pub fn read_history() -> Vec<HistoryEntry> {
    let text = fs::read_to_string(history_path()).unwrap_or_default();
    let mut list: Vec<HistoryEntry> = text
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    list.reverse();
    list
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_fill_defaults() {
        let s: Settings = serde_json::from_str(r#"{"autoUpdate":{"enabled":true}}"#).unwrap();
        assert!(s.auto_update.enabled);
        assert_eq!(s.auto_update.hour, 12);
        assert_eq!(s.auto_update.frequency, Frequency::Daily);
        assert!(s.auto_update.notify);
    }

    #[test]
    fn validates_time() {
        let mut a = AutoUpdate::default();
        assert!(a.is_valid());
        a.hour = 24;
        assert!(!a.is_valid());
        a.hour = 1;
        a.frequency = Frequency::Hours;
        a.every_hours = 3;
        assert!(a.is_valid());
        a.every_hours = 2;
        assert!(!a.is_valid());
    }
}
