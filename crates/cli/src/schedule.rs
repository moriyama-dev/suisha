//! `schedule`: automatic updates on a timer.

use std::io::IsTerminal;

use serde::Serialize;
use suisha_core::managers;
use suisha_core::model::ManagerId;
use suisha_core::privileged;
use suisha_core::schedule as timers;
use suisha_core::store::{self, AutoUpdate, Frequency, HOUR_STEPS};
use suisha_core::system;

use crate::args::Args;
use crate::run::{self, print_json, say, warn};
use crate::text::{self, tr};

const DAYS_EN: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const DAYS_JA: [&str; 7] = ["日", "月", "火", "水", "木", "金", "土"];

/// The schedule and whether it can do its work, for `status`, `schedule`
/// and `--json`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    #[serde(flatten)]
    pub settings: AutoUpdate,
    /// When the timer runs next (RFC 3339).
    pub next: Option<String>,
    /// When the last automatic update ran, from the history (RFC 3339), and
    /// whether it went well.
    pub last: Option<String>,
    pub last_succeeded: Option<bool>,
    /// apt or snap are part of the schedule.
    pub needs_administrator: bool,
    /// This user may update apt and snap without a password.
    pub password_free: bool,
    /// The timer also runs while the user is logged out.
    pub linger: Option<bool>,
    #[serde(skip)]
    next_epoch: Option<i64>,
}

fn needs_administrator(a: &AutoUpdate) -> bool {
    [ManagerId::Apt, ManagerId::Snap].iter().any(|m| {
        managers::binary(*m).is_some() && (a.managers.is_empty() || a.managers.contains(m))
    })
}

pub fn info() -> Info {
    let settings = store::load_settings().auto_update;
    let next_epoch = settings.enabled.then(timers::next_run).flatten();
    let last_run = store::read_history()
        .into_iter()
        .find(|e| e.trigger == "scheduled" && e.action == "update-all");
    Info {
        next: next_epoch.and_then(text::rfc3339_from_epoch),
        last_succeeded: last_run.as_ref().map(|e| e.success),
        last: last_run.map(|e| e.time),
        needs_administrator: needs_administrator(&settings),
        password_free: privileged::auto_update_allowed(),
        linger: timers::linger(),
        settings,
        next_epoch,
    }
}

/// "every day at 03:00"
pub fn describe(a: &AutoUpdate) -> String {
    let (hour, minute) = (a.hour, a.minute);
    match a.frequency {
        Frequency::Hours if a.every_hours <= 1 => tr!(
            "every hour, at {minute:02} minutes past",
            "1 時間ごと（毎時 {minute:02} 分）"
        ),
        Frequency::Hours => {
            let n = a.every_hours;
            tr!(
                "every {n} hours, at {minute:02} minutes past (from midnight)",
                "{n} 時間ごと（0 時から、{minute:02} 分）"
            )
        }
        Frequency::Daily => tr!(
            "every day at {hour:02}:{minute:02}",
            "毎日 {hour:02}:{minute:02}"
        ),
        Frequency::Weekly => {
            let day = usize::from(a.weekday.min(6));
            let (en, ja) = (DAYS_EN[day], DAYS_JA[day]);
            tr!(
                "every {en} at {hour:02}:{minute:02}",
                "毎週{ja}曜日 {hour:02}:{minute:02}"
            )
        }
    }
}

/// The schedule as lines for people.
pub fn lines(info: &Info) -> Vec<String> {
    let a = &info.settings;
    if !a.enabled {
        return vec![tr!(
            "Off. Turn them on with, for example: suisha schedule set daily 03:00",
            "オフです。オンにする例：suisha schedule set daily 03:00"
        )];
    }
    let mut lines = vec![describe(a)];
    let which = if a.managers.is_empty() {
        tr!("all", "すべて")
    } else {
        let names: Vec<&str> = a.managers.iter().map(|m| m.as_str()).collect();
        names.join(", ")
    };
    lines.push(tr!("Package managers: {which}", "対象：{which}"));
    if let Some(next) = info.next_epoch.and_then(text::local_from_epoch) {
        lines.push(tr!("Next run: {next}", "次回：{next}"));
    }
    if let Some(last) = info.last.as_deref().map(text::local_time) {
        let result = match info.last_succeeded {
            Some(false) => tr!(
                " (failed; see `suisha history`)",
                "（失敗。`suisha history` で確認できます）"
            ),
            _ => String::new(),
        };
        lines.push(tr!("Last run: {last}{result}", "前回：{last}{result}"));
    }
    if info.needs_administrator && !info.password_free {
        lines.push(tr!(
            "! apt and snap are skipped: they may not update without a password yet. Run: suisha schedule on",
            "! apt と snap は飛ばされます。パスワードなしで更新する許可がまだありません。実行：suisha schedule on"
        ));
    }
    if info.linger == Some(false) && !system::has_display() {
        lines.push(tr!(
            "! The timer stops while you are logged out. Run: suisha schedule on --linger",
            "! ログアウトしている間はタイマーが止まります。実行：suisha schedule on --linger"
        ));
    }
    lines
}

fn show(args: &Args) -> Result<i32, String> {
    let info = info();
    if args.json {
        print_json(&info);
    } else {
        println!("{}", tr!("Automatic updates", "自動更新"));
        for line in lines(&info) {
            println!("  {line}");
        }
    }
    Ok(0)
}

fn parse_time(text: &str) -> Option<(u8, u8)> {
    let (hour, minute) = text.split_once(':')?;
    let hour: u8 = hour.parse().ok().filter(|h| *h <= 23)?;
    let minute: u8 = (minute.len() == 2)
        .then(|| minute.parse().ok())?
        .filter(|m| *m <= 59)?;
    Some((hour, minute))
}

fn parse_day(text: &str) -> Option<u8> {
    let text = text.to_lowercase();
    (text.len() >= 3)
        .then(|| {
            DAYS_EN
                .iter()
                .position(|day| day.to_lowercase().starts_with(&text))
        })
        .flatten()
        .map(|i| i as u8)
}

fn usage() -> String {
    tr!(
        "Use one of:\n  suisha schedule set daily 03:00\n  suisha schedule set weekly sun 03:00\n  suisha schedule set every 6h        (1h, 3h, 6h or 12h; --minute 30 for half past)\n  suisha schedule on | off",
        "次のどれかの形で指定してください：\n  suisha schedule set daily 03:00\n  suisha schedule set weekly sun 03:00\n  suisha schedule set every 6h        （1h・3h・6h・12h。--minute 30 で毎時 30 分）\n  suisha schedule on | off"
    )
}

/// `daily 03:00`, `weekly sun 03:00`, `hourly`, `every 6h`.
pub fn parse_when(
    words: &[String],
    minute: Option<u8>,
    base: &AutoUpdate,
) -> Result<AutoUpdate, String> {
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let mut a = AutoUpdate {
        enabled: true,
        ..base.clone()
    };
    match words.as_slice() {
        ["daily", time] if minute.is_none() => {
            (a.hour, a.minute) = parse_time(time).ok_or_else(usage)?;
            a.frequency = Frequency::Daily;
        }
        ["weekly", day, time] if minute.is_none() => {
            a.weekday = parse_day(day).ok_or_else(usage)?;
            (a.hour, a.minute) = parse_time(time).ok_or_else(usage)?;
            a.frequency = Frequency::Weekly;
        }
        ["hourly"] => {
            a.frequency = Frequency::Hours;
            a.every_hours = 1;
            a.minute = minute.unwrap_or(0);
        }
        ["every", hours] => {
            a.every_hours = hours
                .trim_end_matches('h')
                .parse()
                .ok()
                .filter(|n| HOUR_STEPS.contains(n))
                .ok_or_else(usage)?;
            a.frequency = Frequency::Hours;
            a.minute = minute.unwrap_or(0);
        }
        _ => return Err(usage()),
    }
    Ok(a)
}

fn apply(args: &Args, a: AutoUpdate) -> Result<i32, String> {
    let mut code = 0;
    let mut settings = store::load_settings();
    let was_enabled = settings.auto_update.enabled;
    let allowed = privileged::auto_update_allowed();

    if a.enabled && needs_administrator(&a) && !allowed && privileged::helpers_installed() {
        say(
            args,
            tr!(
                "Updating apt and snap on a timer needs your password once. It lets your user run\n/usr/lib/suisha/suisha-auto-update, and nothing else, without a password.",
                "apt と snap を予約どおりに更新するために、パスワードを1回だけ入力してください。\nあなたのユーザーが /usr/lib/suisha/suisha-auto-update だけを、パスワードなしで実行できるようになります。"
            ),
        );
        run::sudo_login()?;
    } else if !a.enabled && was_enabled && allowed && !privileged::is_root() {
        if std::io::stdin().is_terminal() {
            say(
                args,
                tr!(
                    "Taking back the permission to update apt and snap without a password needs your password.",
                    "apt と snap をパスワードなしで更新する許可を取り消すために、パスワードを入力してください。"
                ),
            );
        }
        // Optional: without it the permission stays, and the timer is gone anyway.
        let _ = run::sudo_login();
    }

    // A server has nobody logged in most of the time. Without lingering,
    // the user's timers only run during a login session. This comes before
    // the timer is written: lingering also starts systemd for a user who is
    // not logged in at all (a shell from `sudo -i`, for example).
    let linger = timers::linger();
    let wanted = args.linger.unwrap_or(!system::has_display());
    if a.enabled
        && wanted
        && (linger == Some(false) || (linger.is_none() && args.linger == Some(true)))
    {
        match run::sudo_login().and_then(|_| privileged::enable_linger()) {
            Ok(()) => {
                timers::wait_for_user_systemd();
                say(
                    args,
                    tr!(
                        "The timer also runs while you are logged out (lingering was turned on for your user).",
                        "ログアウトしている間もタイマーが動くようにしました（あなたのユーザーの linger を有効にしました）。"
                    ),
                );
            }
            Err(e) => {
                warn(tr!(
                    "Could not keep the timer running while you are logged out: {e}",
                    "ログアウト中もタイマーを動かす設定ができませんでした：{e}"
                ));
                code = 1;
            }
        }
    }

    settings.auto_update = a;
    let (_, warning) = timers::save_auto_update(settings)?;
    if let Some(key) = warning {
        warn(text::message(&key));
        code = 1;
    }

    show(args)?;
    Ok(code)
}

pub fn schedule(args: &Args) -> Result<i32, String> {
    let current = store::load_settings().auto_update;
    let managers = |keep: &AutoUpdate| -> Result<Vec<ManagerId>, String> {
        if args.managers.is_empty() {
            Ok(keep.managers.clone())
        } else {
            crate::packages::chosen(args)
        }
    };
    match args.positional.first().map(String::as_str) {
        None | Some("show") => show(args),
        Some("set") => {
            let mut a = parse_when(&args.positional[1..], args.minute, &current)?;
            // `set` describes the whole schedule: without --manager, everything.
            a.managers = if args.managers.is_empty() {
                Vec::new()
            } else {
                crate::packages::chosen(args)?
            };
            apply(args, a)
        }
        Some("on") => {
            let a = AutoUpdate {
                enabled: true,
                managers: managers(&current)?,
                ..current
            };
            apply(args, a)
        }
        Some("off") => apply(
            args,
            AutoUpdate {
                enabled: false,
                ..current
            },
        ),
        Some(_) => Err(usage()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn when(words: &[&str], minute: Option<u8>) -> Result<AutoUpdate, String> {
        let words: Vec<String> = words.iter().map(|w| w.to_string()).collect();
        parse_when(&words, minute, &AutoUpdate::default())
    }

    #[test]
    fn reads_schedules() {
        let a = when(&["daily", "03:05"], None).unwrap();
        assert!(a.enabled && a.is_valid());
        assert_eq!((a.frequency, a.hour, a.minute), (Frequency::Daily, 3, 5));

        let a = when(&["weekly", "Sun", "23:30"], None).unwrap();
        assert_eq!((a.frequency, a.weekday, a.hour), (Frequency::Weekly, 0, 23));
        assert_eq!(
            when(&["weekly", "saturday", "1:00"], None).unwrap().weekday,
            6
        );

        let a = when(&["every", "6h"], Some(15)).unwrap();
        assert_eq!(
            (a.frequency, a.every_hours, a.minute),
            (Frequency::Hours, 6, 15)
        );
        assert!(a.is_valid());
        let a = when(&["hourly"], None).unwrap();
        assert_eq!((a.every_hours, a.minute), (1, 0));
    }

    #[test]
    fn rejects_schedules_it_cannot_keep() {
        assert!(when(&["daily"], None).is_err());
        assert!(when(&["daily", "24:00"], None).is_err());
        assert!(when(&["daily", "3:5"], None).is_err());
        assert!(when(&["weekly", "03:00"], None).is_err());
        assert!(when(&["weekly", "su", "03:00"], None).is_err());
        // systemd and launchd share the hours 1, 3, 6 and 12.
        assert!(when(&["every", "2h"], None).is_err());
        assert!(when(&["every", "soon"], None).is_err());
        // --minute belongs to the hourly forms.
        assert!(when(&["daily", "03:00"], Some(10)).is_err());
        assert!(when(&[], None).is_err());
    }

    #[test]
    fn describes_schedules() {
        let a = when(&["daily", "03:05"], None).unwrap();
        // Tests run in English unless the environment says otherwise.
        if !text::japanese() {
            assert_eq!(describe(&a), "every day at 03:05");
            let a = when(&["weekly", "mon", "07:00"], None).unwrap();
            assert_eq!(describe(&a), "every Monday at 07:00");
        }
    }
}
