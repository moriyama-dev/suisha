//! `self-update`: keeping Suisha itself up to date.

use suisha_core::schedule as timers;
use suisha_core::self_update;

use crate::args::Args;
use crate::run::{self, print_json, say};
use crate::text::{self, tr};

fn set_auto(args: &Args, enabled: bool) -> Result<i32, String> {
    if enabled && !suisha_core::privileged::allowed(suisha_core::privileged::Helper::SelfUpdate) {
        say(
            args,
            tr!(
                "Updating Suisha by itself needs your password once. It lets your user run\n/usr/lib/suisha/suisha-self-update, and nothing else, without a password.",
                "Suisha が自分で更新するために、パスワードを1回だけ入力してください。\nあなたのユーザーが /usr/lib/suisha/suisha-self-update だけを、パスワードなしで実行できるようになります。"
            ),
        );
        run::sudo_login()?;
    }
    timers::save_self_update(enabled).map_err(|key| text::message(&key))?;
    say(
        args,
        if enabled {
            tr!(
                "Suisha now updates itself once a day.",
                "Suisha は1日に1回、自分で更新するようになりました。"
            )
        } else {
            tr!(
                "Suisha no longer updates itself.",
                "Suisha は自分では更新しなくなりました。"
            )
        },
    );
    Ok(0)
}

pub fn self_update(args: &Args) -> Result<i32, String> {
    if let Some(enabled) = args.auto {
        return set_auto(args, enabled);
    }
    let update = self_update::check().map_err(|e| {
        tr!(
            "Could not ask GitHub for the newest version: {e}",
            "最新の版を GitHub に問い合わせられませんでした：{e}"
        )
    })?;
    if args.json && !update.available {
        print_json(&update);
    }
    let (current, latest) = (&update.current, &update.latest);
    if !update.available {
        say(
            args,
            tr!(
                "Suisha {current} is the newest version.",
                "Suisha {current} は最新の版です。"
            ),
        );
        return Ok(0);
    }
    say(
        args,
        tr!(
            "Suisha {latest} is out (this is {current}).",
            "Suisha {latest} が出ています（今は {current}）。"
        ),
    );
    if update.channel == "manual" {
        let url = &update.releases_url;
        say(
            args,
            tr!(
                "This copy was not installed from a package, so it cannot update itself. Get the new version from:\n  {url}",
                "この Suisha はパッケージから入れたものではないため、自分では更新できません。新しい版はこちら：\n  {url}"
            ),
        );
        return Ok(1);
    }
    let steps = self_update::update_steps(true)?;
    run::execute(args, &steps, "update")
}
