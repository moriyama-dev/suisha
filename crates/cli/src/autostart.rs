//! `autostart`: checking for updates a little after the computer starts or
//! the user logs in.

use serde::Serialize;
use suisha_core::schedule as timers;
use suisha_core::store::{self, StartupCheck};
use suisha_core::system;

use crate::args::Args;
use crate::run::{print_json, say};
use crate::schedule::keep_running_when_logged_out;
use crate::text::{self, tr};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub enabled: bool,
    /// Whether it can run here (on Linux, the suisha-cli package installs it).
    pub supported: bool,
    /// With automatic updates on, the check runs one instead.
    pub auto_update: bool,
    /// The user's timers also run while nobody is logged in, so the check
    /// runs when the computer starts and not only at login.
    pub linger: Option<bool>,
    pub last: Option<StartupCheck>,
}

pub fn info() -> Info {
    let settings = store::load_settings();
    Info {
        enabled: settings.startup_check,
        supported: timers::startup_supported(),
        auto_update: settings.auto_update.enabled,
        linger: timers::linger(),
        last: store::load_startup_check(),
    }
}

/// The check at startup as lines for people.
pub fn lines(info: &Info) -> Vec<String> {
    if !info.supported {
        return vec![tr!(
            "Not installed here: it comes with the suisha-cli package.",
            "このコンピューターには入っていません。suisha-cli のパッケージに含まれています。"
        )];
    }
    if !info.enabled {
        return vec![tr!(
            "Off. Turn it on with: suisha autostart on",
            "オフです。オンにするには：suisha autostart on"
        )];
    }
    let mut lines = vec![if info.auto_update {
        tr!(
            "On: a little after starting, Suisha runs an automatic update.",
            "オン：起動の少しあとに、Suisha が自動更新を1回実行します。"
        )
    } else {
        tr!(
            "On: a little after starting, Suisha checks for updates and says how many are waiting.",
            "オン：起動の少しあとに、Suisha が更新を確認して、何件あるかを知らせます。"
        )
    }];
    if let Some(last) = &info.last {
        let time = text::local_time(&last.time);
        let n = last.updates;
        let found = match n {
            0 => tr!("up to date", "最新でした"),
            _ => {
                let waiting = text::count(n, "update", "updates");
                tr!("{waiting} waiting", "{n} 件の更新がありました")
            }
        };
        lines.push(tr!(
            "Last check: {time}, {found}",
            "前回の確認：{time}、{found}"
        ));
    }
    if info.linger == Some(false) && !system::has_display() {
        lines.push(tr!(
            "! It runs when you log in, not when the computer starts. Run: suisha autostart on --linger",
            "! コンピューターの起動時ではなく、ログインしたときに動きます。実行：suisha autostart on --linger"
        ));
    }
    lines
}

fn show(args: &Args) -> Result<i32, String> {
    let info = info();
    if args.json {
        print_json(&info);
    } else {
        println!("{}", tr!("Check at startup", "起動時の確認"));
        for line in lines(&info) {
            println!("  {line}");
        }
    }
    Ok(0)
}

fn usage() -> String {
    tr!(
        "Use one of:\n  suisha autostart          (show)\n  suisha autostart on | off",
        "次のどれかの形で指定してください：\n  suisha autostart          （表示する）\n  suisha autostart on | off"
    )
}

pub fn autostart(args: &Args) -> Result<i32, String> {
    let enabled = match args.positional.as_slice() {
        [] => return show(args),
        [word] if word == "show" => return show(args),
        [word] if word == "on" => true,
        [word] if word == "off" => false,
        _ => return Err(usage()),
    };
    if enabled && !timers::startup_supported() {
        return Err(lines(&info()).remove(0));
    }
    let mut code = 0;
    if enabled && !keep_running_when_logged_out(args) {
        code = 1;
    }
    timers::save_startup_check(enabled)?;
    say(
        args,
        if enabled {
            tr!(
                "Suisha now checks for updates a little after the computer starts or you log in.",
                "これから Suisha は、コンピューターの起動やログインの少しあとに、更新を確認します。"
            )
        } else {
            tr!(
                "Suisha no longer checks for updates when the computer starts.",
                "Suisha は起動時に更新を確認しなくなりました。"
            )
        },
    );
    if args.json {
        print_json(&info());
    }
    Ok(code)
}
