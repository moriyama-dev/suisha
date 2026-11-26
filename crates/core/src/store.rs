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
        // Turned on without further choices, automatic updates check every
        // hour and install whatever is waiting, for every package manager.
        AutoUpdate {
            enabled: false,
            frequency: Frequency::Hours,
            every_hours: 1,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub auto_update: AutoUpdate,
    /// Update Suisha itself once a day, without asking.
    pub self_update_auto: bool,
    /// The language the window last used ("en" or "ja"), so that background
    /// notifications match it.
    pub language: String,
    /// History entries older than this many days are deleted. `None` keeps
    /// them all; 0 keeps no history.
    pub history_keep_days: Option<u32>,
}

/// The longest period that can be typed in.
pub const HISTORY_KEEP_DAYS_MAX: u32 = 3650;

impl Default for Settings {
    fn default() -> Self {
        Settings {
            auto_update: AutoUpdate::default(),
            self_update_auto: false,
            language: String::new(),
            history_keep_days: Some(100),
        }
    }
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

/// A ceiling on the file size, whatever the period: hourly automatic updates
/// write about 17,500 lines a year.
const HISTORY_KEEP: usize = 20_000;

fn history_path() -> PathBuf {
    data_dir().join("history.jsonl")
}

pub fn append_history(entries: &[HistoryEntry]) {
    if entries.is_empty() {
        return;
    }
    let keep_days = load_settings().history_keep_days;
    if keep_days == Some(0) {
        // The user chose to keep no history.
        prune_history(keep_days);
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
    prune_history(keep_days);
}

/// The history without entries older than `keep_days` (`None` keeps every
/// age, 0 keeps nothing) and without more than `max` lines. `None` when
/// nothing has to go. Lines whose time cannot be read are kept, except when
/// nothing is kept at all.
fn pruned(
    text: &str,
    keep_days: Option<u32>,
    now: chrono::DateTime<chrono::Utc>,
    max: usize,
) -> Option<String> {
    if keep_days == Some(0) {
        return (!text.is_empty()).then(String::new);
    }
    let cutoff = keep_days.map(|d| now - chrono::Duration::days(i64::from(d)));
    let total = text.lines().count();
    let mut kept: Vec<&str> = text
        .lines()
        .filter(|line| {
            let Some(cutoff) = cutoff else { return true };
            serde_json::from_str::<HistoryEntry>(line)
                .ok()
                .and_then(|e| chrono::DateTime::parse_from_rfc3339(&e.time).ok())
                .is_none_or(|t| t >= cutoff)
        })
        .collect();
    if kept.len() > max {
        kept.drain(..kept.len() - max);
    }
    if kept.len() == total {
        return None;
    }
    Some(kept.iter().map(|l| format!("{l}\n")).collect())
}

/// Delete history entries older than `keep_days` days.
pub fn prune_history(keep_days: Option<u32>) {
    let Ok(text) = fs::read_to_string(history_path()) else {
        return;
    };
    let Some(kept) = pruned(&text, keep_days, chrono::Utc::now(), HISTORY_KEEP) else {
        return;
    };
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
        assert_eq!(s.auto_update.frequency, Frequency::Hours);
        assert_eq!(s.auto_update.every_hours, 1);
        assert!(s.auto_update.managers.is_empty());
        assert!(s.auto_update.notify);
    }

    #[test]
    fn history_period_defaults_to_100_days() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.history_keep_days, Some(100));
        // null means "keep forever", 0 means "keep nothing".
        let s: Settings = serde_json::from_str(r#"{"historyKeepDays":null}"#).unwrap();
        assert_eq!(s.history_keep_days, None);
        let s: Settings = serde_json::from_str(r#"{"historyKeepDays":0}"#).unwrap();
        assert_eq!(s.history_keep_days, Some(0));
    }

    fn line(time: &str) -> String {
        serde_json::to_string(&HistoryEntry {
            time: time.into(),
            trigger: "manual".into(),
            action: "update-all".into(),
            managers: vec![ManagerId::Flatpak],
            packages: vec![],
            success: true,
            message: String::new(),
        })
        .unwrap()
    }

    #[test]
    fn prunes_old_history() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-11-20T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let old = line("2026-10-01T12:00:00Z");
        let edge = line("2026-10-21T12:00:00Z");
        let new = line("2026-11-20T11:00:00Z");
        let text = format!("{old}\nnot json\n{edge}\n{new}\n");

        // 30 days: the entry exactly 30 days old stays, unreadable lines stay.
        assert_eq!(
            pruned(&text, Some(30), now, 100).unwrap(),
            format!("not json\n{edge}\n{new}\n")
        );
        assert_eq!(
            pruned(&text, Some(3), now, 100).unwrap(),
            format!("not json\n{new}\n")
        );
        // Nothing to delete: leave the file alone.
        assert!(pruned(&text, None, now, 100).is_none());
        assert!(pruned(&text, Some(400), now, 100).is_none());
        // The line limit applies even when every age is kept.
        assert_eq!(
            pruned(&text, None, now, 2).unwrap(),
            format!("{edge}\n{new}\n")
        );
        // Keeping no history empties the file, once.
        assert_eq!(pruned(&text, Some(0), now, 100).unwrap(), "");
        assert!(pruned("", Some(0), now, 100).is_none());
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
