//! Scheduling Suisha's background work with the system's own timers:
//! systemd user timers on Linux and launchd agents on macOS. Suisha itself
//! does not stay running: a timer starts `suisha --auto-update` (or
//! `suisha --self-update`) at the chosen times.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::exec::{find_bin, run, run_with_env};
use crate::store::{AutoUpdate, Frequency};

const AUTO_UNIT: &str = "suisha-auto-update";
const SELF_UNIT: &str = "suisha-self-update";
pub const AUTO_LABEL: &str = "dev.moriyama.suisha.auto-update";
pub const SELF_LABEL: &str = "dev.moriyama.suisha.self-update";

/// The program the timer should start. Inside an AppImage the running file
/// is a temporary mount, so the AppImage itself is used.
pub fn program_path() -> Result<PathBuf, String> {
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        return Ok(PathBuf::from(appimage));
    }
    std::env::current_exe().map_err(|e| e.to_string())
}

const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// The systemd calendar expression for the automatic update schedule.
pub fn calendar(a: &AutoUpdate) -> String {
    match a.frequency {
        Frequency::Hours if a.every_hours <= 1 => format!("*-*-* *:{:02}:00", a.minute),
        Frequency::Hours => format!("*-*-* 00/{}:{:02}:00", a.every_hours, a.minute),
        Frequency::Daily => format!("*-*-* {:02}:{:02}:00", a.hour, a.minute),
        Frequency::Weekly => format!(
            "{} *-*-* {:02}:{:02}:00",
            DAYS[a.weekday.min(6) as usize],
            a.hour,
            a.minute
        ),
    }
}

pub fn systemd_units(
    program: &str,
    argument: &str,
    description: &str,
    calendar: &str,
    spread: &str,
) -> (String, String) {
    // systemd expands "%" specifiers; "%%" is a literal percent sign.
    let program = program.replace('%', "%%").replace('"', "\\\"");
    let service = format!(
        "[Unit]\n\
         Description={description}\n\
         After=network-online.target\n\
         \n\
         [Service]\n\
         Type=oneshot\n\
         ExecStart=\"{program}\" {argument}\n"
    );
    let timer = format!(
        "[Unit]\n\
         Description={description} (timer)\n\
         \n\
         [Timer]\n\
         OnCalendar={calendar}\n\
         Persistent=true\n\
         RandomizedDelaySec={spread}\n\
         \n\
         [Install]\n\
         WantedBy=timers.target\n"
    );
    (service, timer)
}

/// One `StartCalendarInterval` entry: pairs of key (Weekday, Hour, Minute)
/// and value.
pub type CalendarEntry = Vec<(&'static str, u32)>;

/// The launchd calendar entries for the automatic update schedule. Every
/// few hours becomes one entry per hour, at the same hours systemd uses.
pub fn launchd_calendar(a: &AutoUpdate) -> Vec<CalendarEntry> {
    let minute = u32::from(a.minute);
    match a.frequency {
        Frequency::Hours if a.every_hours <= 1 => vec![vec![("Minute", minute)]],
        Frequency::Hours => (0..24)
            .step_by(usize::from(a.every_hours))
            .map(|h| vec![("Hour", h as u32), ("Minute", minute)])
            .collect(),
        Frequency::Daily => vec![vec![("Hour", u32::from(a.hour)), ("Minute", minute)]],
        Frequency::Weekly => vec![vec![
            ("Weekday", u32::from(a.weekday.min(6))),
            ("Hour", u32::from(a.hour)),
            ("Minute", minute),
        ]],
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A launchd agent that starts `program argument` at the given times. When
/// the Mac was asleep, launchd starts it once after waking up.
pub fn launchd_plist(
    label: &str,
    program: &str,
    argument: &str,
    calendar: &[CalendarEntry],
    log: &str,
) -> String {
    let mut when = String::new();
    for entry in calendar {
        when.push_str("    <dict>\n");
        for (key, value) in entry {
            when.push_str(&format!(
                "      <key>{key}</key>\n      <integer>{value}</integer>\n"
            ));
        }
        when.push_str("    </dict>\n");
    }
    let program = xml_escape(program);
    let log = xml_escape(log);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{program}</string>
    <string>{argument}</string>
  </array>
  <key>StartCalendarInterval</key>
  <array>
{when}  </array>
  <key>ProcessType</key>
  <string>Background</string>
  <key>StandardOutPath</key>
  <string>{log}</string>
  <key>StandardErrorPath</key>
  <string>{log}</string>
</dict>
</plist>
"#
    )
}

fn write_if_changed(path: &PathBuf, content: &str) -> Result<bool, String> {
    if fs::read_to_string(path).ok().as_deref() == Some(content) {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(true)
}

fn systemd_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("systemd/user")
}

fn systemctl(args: &[&str]) -> Result<(), String> {
    let systemctl = find_bin("systemctl").ok_or("systemctl was not found")?;
    let mut full = vec!["--user"];
    full.extend_from_slice(args);
    // Talk to the user's own systemd through its standard bus, even when the
    // app was started with a different session bus.
    let mut envs = Vec::new();
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            use std::os::unix::fs::MetadataExt;
            let uid = fs::metadata("/proc/self").ok()?.uid();
            Some(PathBuf::from(format!("/run/user/{uid}")))
        });
    if let Some(bus) = runtime.map(|r| r.join("bus")).filter(|b| b.exists()) {
        envs.push((
            "DBUS_SESSION_BUS_ADDRESS",
            format!("unix:path={}", bus.display()),
        ));
    }
    let out = run_with_env(&systemctl, &full, &envs, Duration::from_secs(30));
    if out.success { Ok(()) } else { Err(out.tail()) }
}

fn launch_agent_path(label: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library/LaunchAgents")
        .join(format!("{label}.plist"))
}

/// Where the agents write their output: ~/Library/Logs/Suisha.
/// launchd does not create the folder itself.
fn log_path(name: &str) -> PathBuf {
    let dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library/Logs/Suisha");
    let _ = fs::create_dir_all(&dir);
    dir.join(format!("{name}.log"))
}

fn gui_domain() -> Result<String, String> {
    let id = find_bin("id").ok_or("id was not found")?;
    let out = run(&id, &["-u"], Duration::from_secs(5));
    let uid = out.stdout.trim();
    if !out.success || uid.is_empty() {
        return Err(out.tail());
    }
    Ok(format!("gui/{uid}"))
}

/// Install, update or remove one launchd agent. `plist` is `None` to remove it.
fn set_agent(label: &str, plist: Option<String>) -> Result<(), String> {
    let path = launch_agent_path(label);
    let launchctl = find_bin("launchctl").ok_or("launchctl was not found")?;
    let domain = gui_domain()?;
    let file = path.to_string_lossy().into_owned();
    let timeout = Duration::from_secs(20);
    match plist {
        Some(text) => {
            let changed = write_if_changed(&path, &text)?;
            let loaded = run(
                &launchctl,
                &["print", &format!("{domain}/{label}")],
                timeout,
            )
            .success;
            if changed || !loaded {
                // Load it again so that launchd reads the new times.
                let _ = run(&launchctl, &["bootout", &domain, &file], timeout);
                let out = run(&launchctl, &["bootstrap", &domain, &file], timeout);
                if !out.success {
                    return Err(out.tail());
                }
            }
        }
        None => {
            if path.exists() {
                let _ = run(&launchctl, &["bootout", &domain, &file], timeout);
                fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

/// Install, update or remove one timer. `units` is `None` to remove it.
fn set_timer(unit: &str, units: Option<(String, String)>) -> Result<(), String> {
    let dir = systemd_dir();
    let service = dir.join(format!("{unit}.service"));
    let timer = dir.join(format!("{unit}.timer"));
    let timer_name = format!("{unit}.timer");
    match units {
        Some((s, t)) => {
            let changed = write_if_changed(&service, &s)? | write_if_changed(&timer, &t)?;
            if changed {
                systemctl(&["daemon-reload"])?;
            }
            systemctl(&["enable", "--now", &timer_name])?;
            if changed {
                // Pick up a new time for a timer that was already running.
                systemctl(&["restart", &timer_name])?;
            }
        }
        None => {
            if timer.exists() || service.exists() {
                let _ = systemctl(&["disable", "--now", &timer_name]);
                let _ = fs::remove_file(&timer);
                let _ = fs::remove_file(&service);
                let _ = systemctl(&["daemon-reload"]);
            }
        }
    }
    Ok(())
}

/// Make the automatic update timer match the settings.
pub fn apply(a: &AutoUpdate) -> Result<(), String> {
    let program = program_path()?;
    if cfg!(target_os = "macos") {
        let plist = a.enabled.then(|| {
            launchd_plist(
                AUTO_LABEL,
                &program.to_string_lossy(),
                "--auto-update",
                &launchd_calendar(a),
                &log_path("auto-update").to_string_lossy(),
            )
        });
        return set_agent(AUTO_LABEL, plist);
    }
    let units = a.enabled.then(|| {
        systemd_units(
            &program.to_string_lossy(),
            "--auto-update",
            "Suisha automatic updates",
            &calendar(a),
            "5min",
        )
    });
    set_timer(AUTO_UNIT, units)
}

/// Turn the daily update of Suisha itself on or off.
pub fn apply_self_update(enabled: bool) -> Result<(), String> {
    let program = program_path()?;
    if cfg!(target_os = "macos") {
        let plist = enabled.then(|| {
            launchd_plist(
                SELF_LABEL,
                &program.to_string_lossy(),
                "--self-update",
                &[vec![("Hour", 12), ("Minute", 30)]],
                &log_path("self-update").to_string_lossy(),
            )
        });
        return set_agent(SELF_LABEL, plist);
    }
    let units = enabled.then(|| {
        systemd_units(
            &program.to_string_lossy(),
            "--self-update",
            "Suisha updating itself",
            "daily",
            "2h",
        )
    });
    set_timer(SELF_UNIT, units)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendars() {
        let mut a = AutoUpdate {
            enabled: true,
            hour: 7,
            minute: 5,
            ..Default::default()
        };
        assert_eq!(calendar(&a), "*-*-* 07:05:00");
        a.frequency = Frequency::Weekly;
        a.weekday = 3;
        assert_eq!(calendar(&a), "Wed *-*-* 07:05:00");
        a.frequency = Frequency::Hours;
        a.every_hours = 1;
        assert_eq!(calendar(&a), "*-*-* *:05:00");
        a.every_hours = 6;
        assert_eq!(calendar(&a), "*-*-* 00/6:05:00");
    }

    #[test]
    fn unit_text() {
        let (service, timer) = systemd_units(
            "/opt/50%/suisha",
            "--auto-update",
            "Suisha automatic updates",
            "*-*-* 07:05:00",
            "5min",
        );
        assert!(service.contains("ExecStart=\"/opt/50%%/suisha\" --auto-update\n"));
        assert!(timer.contains("OnCalendar=*-*-* 07:05:00\n"));
        assert!(timer.contains("Persistent=true\n"));
    }

    #[test]
    fn launchd_times() {
        let mut a = AutoUpdate {
            enabled: true,
            hour: 7,
            minute: 5,
            ..Default::default()
        };
        assert_eq!(launchd_calendar(&a), vec![vec![("Hour", 7), ("Minute", 5)]]);
        a.frequency = Frequency::Weekly;
        a.weekday = 0;
        assert_eq!(
            launchd_calendar(&a),
            vec![vec![("Weekday", 0), ("Hour", 7), ("Minute", 5)]]
        );
        a.frequency = Frequency::Hours;
        a.every_hours = 1;
        assert_eq!(launchd_calendar(&a), vec![vec![("Minute", 5)]]);
        a.every_hours = 6;
        let hours: Vec<u32> = launchd_calendar(&a).iter().map(|e| e[0].1).collect();
        assert_eq!(hours, vec![0, 6, 12, 18]);
    }

    #[test]
    fn launchd_plist_text() {
        let plist = launchd_plist(
            AUTO_LABEL,
            "/Applications/A&B.app/Contents/MacOS/suisha",
            "--auto-update",
            &[vec![("Hour", 21), ("Minute", 30)]],
            "/Users/me/Library/Logs/Suisha/auto-update.log",
        );
        assert!(plist.contains("<string>dev.moriyama.suisha.auto-update</string>"));
        assert!(plist.contains("<string>/Applications/A&amp;B.app/Contents/MacOS/suisha</string>"));
        assert!(plist.contains("<string>--auto-update</string>"));
        assert!(plist.contains(
            "    <dict>\n      <key>Hour</key>\n      <integer>21</integer>\n      <key>Minute</key>\n      <integer>30</integer>\n    </dict>\n"
        ));
    }
}
