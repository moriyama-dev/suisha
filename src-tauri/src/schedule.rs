//! Scheduling Suisha's background work with systemd user timers. Suisha
//! itself does not stay running: a timer starts `suisha --auto-update` (or
//! `suisha --self-update`) at the chosen times.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::exec::{find_bin, run_with_env};
use crate::store::{AutoUpdate, Frequency};

const AUTO_UNIT: &str = "suisha-auto-update";
const SELF_UNIT: &str = "suisha-self-update";

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
}
