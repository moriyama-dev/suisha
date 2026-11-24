//! The language of the messages (English or Japanese) and how times and
//! message keys are shown.

use std::sync::OnceLock;

use chrono::{DateTime, Local, Utc};

static JAPANESE: OnceLock<bool> = OnceLock::new();

/// The first locale variable that is set decides, as in every other program.
/// `SUISHA_LANG` (`en` or `ja`) overrides them.
pub fn language_from(get: impl Fn(&str) -> Option<String>) -> &'static str {
    let chosen = ["SUISHA_LANG", "LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| get(name))
        .find(|value| !value.is_empty())
        .unwrap_or_default();
    if chosen.starts_with("ja") { "ja" } else { "en" }
}

pub fn japanese() -> bool {
    *JAPANESE.get_or_init(|| language_from(|name| std::env::var(name).ok()) == "ja")
}

pub fn language() -> &'static str {
    if japanese() { "ja" } else { "en" }
}

/// A message in English and in Japanese; both take the same arguments.
macro_rules! tr {
    ($en:literal, $ja:literal $(, $($arg:tt)*)?) => {
        if $crate::text::japanese() {
            format!($ja $(, $($arg)*)?)
        } else {
            format!($en $(, $($arg)*)?)
        }
    };
}
pub(crate) use tr;

/// "1 update" or "3 updates".
pub fn count(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// A time from the history (RFC 3339) in this computer's time zone.
pub fn local_time(rfc3339: &str) -> String {
    DateTime::parse_from_rfc3339(rfc3339)
        .map(|t| t.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| rfc3339.to_string())
}

/// Seconds since 1970 in this computer's time zone.
pub fn local_from_epoch(seconds: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .map(|t| t.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
}

/// Seconds since 1970 as RFC 3339, for `--json`.
pub fn rfc3339_from_epoch(seconds: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

/// The text for a message key the shared code returns. Anything that is not
/// a key (the last lines a package manager printed, for example) is shown as
/// it is.
pub fn message(key: &str) -> String {
    if let Some(detail) = key.strip_prefix("auto.accessFailed:") {
        return tr!(
            "Could not allow password-less updates for apt and snap: {detail}",
            "apt と snap をパスワードなしで更新する許可を設定できませんでした：{detail}"
        );
    }
    if let Some(os) = key.strip_prefix("setup.otherOs:") {
        return tr!(
            "This list was saved on {os}. Apps come from different places there, so Suisha only opens lists saved on the same kind of system.",
            "この一覧は {os} で保存されたものです。アプリの入手先が違うため、Suisha で開けるのは同じ種類のシステムで保存した一覧だけです。"
        );
    }
    match key {
        "auto.noHelpers" => tr!(
            "apt and snap will not be updated automatically: install Suisha from its .deb package to enable this.",
            "apt と snap は自動では更新されません。使うには、Suisha を .deb パッケージから入れてください。"
        ),
        "auto.accessCancelled" => tr!(
            "The password was not entered, so apt and snap will not be updated automatically. The other package managers will.",
            "パスワードが入力されなかったため、apt と snap は自動では更新されません。ほかのパッケージマネージャーは自動で更新されます。"
        ),
        "history.adminSkipped" => tr!(
            "apt and snap were skipped: they are not allowed to update without a password. Run `suisha schedule on` and enter your password when asked.",
            "apt と snap は飛ばしました。パスワードなしで更新する許可がありません。`suisha schedule on` を実行し、パスワードを求められたら入力してください。"
        ),
        "self.noBrew" => tr!(
            "Install Suisha with Homebrew to let it update itself.",
            "Suisha が自分で更新するには、Homebrew で Suisha を入れてください。"
        ),
        "setup.tooLarge" => tr!(
            "The file is too large to be a Suisha list.",
            "ファイルが大きすぎます。Suisha の一覧ではないようです。"
        ),
        "setup.notASetupFile" => tr!(
            "This file is not a list saved by Suisha.",
            "このファイルは Suisha で保存した一覧ではありません。"
        ),
        "setup.newerVersion" => tr!(
            "This list was saved by a newer version of Suisha. Update Suisha and try again.",
            "新しい版の Suisha で保存された一覧です。Suisha を更新してから開いてください。"
        ),
        "cancelled" => tr!(
            "The password was not entered.",
            "パスワードが入力されませんでした。"
        ),
        other => other.to_string(),
    }
}

/// Print rows in columns. Every column but the last is padded to its widest
/// cell; rows may be shorter than the widest row.
pub fn table(rows: &[Vec<String>]) -> String {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![0; columns];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let mut out = String::new();
    for row in rows {
        let mut line = String::new();
        for (i, cell) in row.iter().enumerate() {
            line.push_str(cell);
            if i + 1 < row.len() {
                let pad = widths[i].saturating_sub(cell.chars().count()) + 2;
                line.extend(std::iter::repeat_n(' ', pad));
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn picks_the_language_from_the_locale() {
        assert_eq!(language_from(env(&[("LANG", "ja_JP.UTF-8")])), "ja");
        assert_eq!(language_from(env(&[("LANG", "en_CA.UTF-8")])), "en");
        assert_eq!(language_from(env(&[])), "en");
        // LC_ALL wins over LANG, and an empty variable counts as unset.
        assert_eq!(
            language_from(env(&[("LC_ALL", "C"), ("LANG", "ja_JP.UTF-8")])),
            "en"
        );
        assert_eq!(
            language_from(env(&[("LC_ALL", ""), ("LANG", "ja_JP.UTF-8")])),
            "ja"
        );
        assert_eq!(
            language_from(env(&[("SUISHA_LANG", "ja"), ("LANG", "en_US.UTF-8")])),
            "ja"
        );
    }

    #[test]
    fn counts() {
        assert_eq!(count(1, "update", "updates"), "1 update");
        assert_eq!(count(0, "update", "updates"), "0 updates");
        assert_eq!(count(12, "update", "updates"), "12 updates");
    }

    #[test]
    fn columns_line_up() {
        let rows = vec![
            vec!["apt:git".to_string(), "2.43".into(), "Git".into()],
            vec!["snap:lxd".to_string(), "5".into()],
        ];
        assert_eq!(table(&rows), "apt:git   2.43  Git\nsnap:lxd  5\n");
    }

    #[test]
    fn unknown_keys_are_shown_as_they_are() {
        assert_eq!(
            message("E: Unable to locate package x"),
            "E: Unable to locate package x"
        );
        assert!(message("setup.otherOs:macos").contains("macos"));
    }
}
