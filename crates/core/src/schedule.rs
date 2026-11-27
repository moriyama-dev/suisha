//! Scheduling Suisha's background work with the system's own timers:
//! systemd user timers on Linux and launchd agents on macOS. Suisha itself
//! does not stay running: a timer starts `suisha --auto-update` (or
//! `suisha --self-update`) at the chosen times.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::exec::{find_bin, run, run_with_env};
use crate::managers;
use crate::model::ManagerId;
use crate::privileged;
use crate::self_update;
use crate::store::{self, AutoUpdate, Frequency, Settings};

const AUTO_UNIT: &str = "suisha-auto-update";
const SELF_UNIT: &str = "suisha-self-update";
pub const AUTO_LABEL: &str = "dev.moriyama.suisha.auto-update";
pub const SELF_LABEL: &str = "dev.moriyama.suisha.self-update";
pub const STARTUP_LABEL: &str = "dev.moriyama.suisha.startup-check";
/// On Linux the suisha-cli package installs the check at startup for every
/// user (systemd's own place for packaged user units, already enabled), and
/// the setting decides whether it does anything.
pub const STARTUP_TIMER: &str = "/usr/lib/systemd/user/suisha-startup-check.timer";

/// The program the timer should start. Inside an AppImage the running file
/// is a temporary mount, so the AppImage itself is used.
pub fn program_path() -> Result<PathBuf, String> {
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        return Ok(PathBuf::from(appimage));
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(command_next_to(&exe).unwrap_or(exe))
}

/// On Linux the window is `suisha-gui`, and the `suisha` command is
/// installed next to it. Timers start the command: it does the same work
/// and does not load the libraries a window needs.
fn command_next_to(exe: &Path) -> Option<PathBuf> {
    if exe.file_name()? != "suisha-gui" {
        return None;
    }
    let command = exe.with_file_name("suisha");
    command.is_file().then_some(command)
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

/// A launchd agent that starts `program argument` each time the user logs
/// in (and once when it is loaded).
pub fn launchd_login_plist(label: &str, program: &str, argument: &str, log: &str) -> String {
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
  <key>RunAtLoad</key>
  <true/>
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

/// Run `systemctl --user` and return what it printed.
fn systemctl(args: &[&str]) -> Result<String, String> {
    let systemctl = find_bin("systemctl").ok_or("systemctl was not found")?;
    let mut full = vec!["--user"];
    full.extend_from_slice(args);
    // Times are printed in UTC, so that they can be read back whatever the
    // time zone is called here.
    let mut envs = vec![("TZ", "UTC".to_string())];
    // Talk to the user's own systemd through its standard bus, even when the
    // app was started with a different session bus.
    if let Some(bus) = user_bus().filter(|b| b.exists()) {
        envs.push((
            "DBUS_SESSION_BUS_ADDRESS",
            format!("unix:path={}", bus.display()),
        ));
    }
    let out = run_with_env(&systemctl, &full, &envs, Duration::from_secs(30));
    if out.success {
        Ok(out.stdout)
    } else if out.stderr.contains("Failed to connect to bus") {
        // A shell from `su` or `sudo -i`, or a container without a login.
        Err(
            "systemd is not running for this user, so there is nothing to keep the timer. \
             Log in as this user (for example over SSH) and try again, or first run: \
             sudo loginctl enable-linger \"$USER\""
                .into(),
        )
    } else {
        Err(out.tail())
    }
}

/// The socket of this user's own systemd and D-Bus.
fn user_bus() -> Option<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let uid = rustix::process::getuid().as_raw();
            PathBuf::from(format!("/run/user/{uid}"))
        });
    Some(runtime.join("bus"))
}

/// After lingering was turned on for a user without a login session,
/// systemd starts for that user; wait until it answers.
pub fn wait_for_user_systemd() {
    let Some(bus) = user_bus() else { return };
    for _ in 0..50 {
        if bus.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// `systemctl show --property=NextElapseUSecRealtime` with `TZ=UTC`:
/// `NextElapseUSecRealtime=Wed 2026-11-25 03:02:02 UTC`, in seconds since
/// 1970. The value is empty while the timer is not waiting for anything.
pub fn parse_next_run(text: &str) -> Option<i64> {
    let value = text
        .lines()
        .find_map(|l| l.strip_prefix("NextElapseUSecRealtime="))?;
    let mut words = value.split_whitespace();
    let (_day, date, time, zone) = (words.next()?, words.next()?, words.next()?, words.next()?);
    if zone != "UTC" {
        return None;
    }
    chrono::NaiveDateTime::parse_from_str(&format!("{date} {time}"), "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|t| t.and_utc().timestamp())
}

/// When the automatic update timer runs next, in seconds since 1970
/// (Linux; `None` where systemd does not say).
pub fn next_run() -> Option<i64> {
    if cfg!(target_os = "macos") {
        return None;
    }
    systemctl(&[
        "show",
        &format!("{AUTO_UNIT}.timer"),
        "--property=NextElapseUSecRealtime",
    ])
    .ok()
    .and_then(|text| parse_next_run(&text))
}

/// Whether this user's timers also run while the user is logged out
/// (`loginctl enable-linger`). `None` when the system does not say, and on
/// macOS, where launchd agents belong to the login session.
pub fn linger() -> Option<bool> {
    if cfg!(target_os = "macos") {
        return None;
    }
    let loginctl = find_bin("loginctl")?;
    let uid = rustix::process::getuid().as_raw().to_string();
    let out = run(
        &loginctl,
        &["show-user", &uid, "--property=Linger", "--value"],
        Duration::from_secs(20),
    );
    match out.stdout.trim() {
        "yes" if out.success => Some(true),
        "no" if out.success => Some(false),
        // systemd knows nothing about a user who has no login session:
        // "User ID 0 is not logged in or lingering".
        _ if out.stderr.contains("not logged in or lingering") => Some(false),
        _ => None,
    }
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

/// Whether the check at startup can run on this computer.
pub fn startup_supported() -> bool {
    cfg!(target_os = "macos") || Path::new(STARTUP_TIMER).is_file()
}

/// Written just before the login agent is loaded: launchd starts an agent
/// with RunAtLoad as soon as it is loaded, and that run is not a login.
pub fn startup_marker() -> PathBuf {
    store::data_dir().join("startup-agent-loaded")
}

/// Make the check at startup match the setting. Only macOS has something to
/// install; on Linux the package already did.
pub fn apply_startup_check(enabled: bool) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Ok(());
    }
    let program = program_path()?;
    let plist = enabled.then(|| {
        launchd_login_plist(
            STARTUP_LABEL,
            &program.to_string_lossy(),
            "--startup-check",
            &log_path("startup-check").to_string_lossy(),
        )
    });
    if plist.is_some() {
        // The window also calls this when it opens; it has just shown the
        // updates, so skipping a check that starts right then loses nothing.
        let marker = startup_marker();
        if let Some(dir) = marker.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(&marker, "");
    }
    set_agent(STARTUP_LABEL, plist)
}

/// Turn the check at startup on or off.
pub fn save_startup_check(enabled: bool) -> Result<Settings, String> {
    apply_startup_check(enabled)?;
    let mut settings = store::load_settings();
    settings.startup_check = enabled;
    settings.startup_check_noticed = true;
    store::save_settings(&settings)?;
    Ok(settings)
}

/// Save the automatic update settings and make the timer match them.
///
/// Turning automatic updates on for apt or snap asks once for the password,
/// so that later runs need none. The second value is a message key for
/// something the person should be told (the settings are saved all the same).
pub fn save_auto_update(settings: Settings) -> Result<(Settings, Option<String>), String> {
    if !settings.auto_update.is_valid() {
        return Err("Invalid time".into());
    }
    let previous = store::load_settings();
    let mut warning = None;

    let a = &settings.auto_update;
    let wants_admin = [ManagerId::Apt, ManagerId::Snap].iter().any(|m| {
        managers::binary(*m).is_some() && (a.managers.is_empty() || a.managers.contains(m))
    });
    let turning_on = a.enabled && wants_admin;
    let turning_off = !a.enabled && previous.auto_update.enabled;
    if turning_on && !privileged::auto_update_allowed() {
        if !privileged::helpers_installed() {
            warning = Some("auto.noHelpers".into());
        } else if let Err(e) = privileged::set_auto_update_access(true) {
            warning = Some(if e == "cancelled" {
                "auto.accessCancelled".into()
            } else {
                format!("auto.accessFailed:{e}")
            });
        }
    } else if turning_off && privileged::auto_update_allowed() {
        // Taking the permission back is optional; dismissing the
        // dialog leaves it in place, and the timer is gone anyway.
        let _ = privileged::set_auto_update_access(false);
    }

    apply(&settings.auto_update)?;
    // Only the schedule is edited here; keep what this does not know about.
    let settings = Settings {
        self_update_auto: previous.self_update_auto,
        startup_check: previous.startup_check,
        startup_check_noticed: previous.startup_check_noticed,
        ..settings
    };
    store::save_settings(&settings)?;
    Ok((settings, warning))
}

/// Turn the daily update of Suisha itself on or off. Turning it on asks for
/// the password once, unless that was done before.
pub fn save_self_update(enabled: bool) -> Result<Settings, String> {
    if enabled && cfg!(target_os = "macos") {
        if self_update::channel() != "brew" {
            return Err("self.noBrew".into());
        }
    } else if enabled {
        if !privileged::helpers_installed() {
            return Err("auto.noHelpers".into());
        }
        if !privileged::allowed(privileged::Helper::SelfUpdate) {
            privileged::set_access(privileged::Helper::SelfUpdate, true)?;
        }
    }
    apply_self_update(enabled)?;
    let mut settings = store::load_settings();
    settings.self_update_auto = enabled;
    store::save_settings(&settings)?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timers_start_the_command_not_the_window() {
        let dir = std::env::temp_dir().join(format!("suisha-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let window = dir.join("suisha-gui");
        // Without the command next to it, the window starts itself.
        assert_eq!(command_next_to(&window), None);
        fs::write(dir.join("suisha"), "").unwrap();
        assert_eq!(command_next_to(&window), Some(dir.join("suisha")));
        assert_eq!(command_next_to(&dir.join("suisha")), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_login_agent_runs_at_load() {
        let plist = launchd_login_plist(
            STARTUP_LABEL,
            "/Applications/A & B.app/x",
            "--startup-check",
            "/tmp/l.log",
        );
        assert!(plist.contains("<key>RunAtLoad</key>\n  <true/>"));
        assert!(plist.contains("<string>/Applications/A &amp; B.app/x</string>"));
        assert!(plist.contains("<string>--startup-check</string>"));
        assert!(!plist.contains("StartCalendarInterval"));
    }

    #[test]
    fn next_run_from_systemctl() {
        let text = "NextElapseUSecRealtime=Wed 2026-11-25 03:02:02 UTC\n";
        assert_eq!(parse_next_run(text), Some(1795575722));
        // A stopped timer has nothing coming up.
        assert_eq!(parse_next_run("NextElapseUSecRealtime=\n"), None);
        assert_eq!(parse_next_run(""), None);
        // Only the format asked for is trusted.
        let local = "NextElapseUSecRealtime=Wed 2026-11-25 03:02:02 EST\n";
        assert_eq!(parse_next_run(local), None);
    }

    #[test]
    fn calendars() {
        let mut a = AutoUpdate {
            enabled: true,
            frequency: Frequency::Daily,
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
            frequency: Frequency::Daily,
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
