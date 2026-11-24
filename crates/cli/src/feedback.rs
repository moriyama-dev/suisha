//! `report` and `request`: a new GitHub issue with the details filled in.
//!
//! Only the Suisha version, the system, the package managers, the language
//! and the time go into the issue. No names, paths or package lists.

use std::time::Duration;

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday};
use suisha_core::exec::{find_bin, run};
use suisha_core::managers;
use suisha_core::setup_file;
use suisha_core::system;

use crate::args::Args;
use crate::run::print_json;
use crate::text::{self, tr};

pub const REPO_URL: &str = "https://github.com/moriyama-dev/suisha";

/// The time in Toronto, which the project works in: Eastern time, with
/// daylight time from the second Sunday of March to the first Sunday of
/// November (both changes at 2:00 local time).
pub fn toronto(utc: DateTime<Utc>) -> (NaiveDateTime, &'static str) {
    let change = |month: u32, sunday: u8, utc_hour: u32| {
        NaiveDate::from_weekday_of_month_opt(utc.year(), month, Weekday::Sun, sunday)
            .and_then(|d| d.and_hms_opt(utc_hour, 0, 0))
            .map(|t| Utc.from_utc_datetime(&t))
    };
    // 2:00 EST is 7:00 UTC; 2:00 EDT is 6:00 UTC.
    let daylight = match (change(3, 2, 7), change(11, 1, 6)) {
        (Some(start), Some(end)) => utc >= start && utc < end,
        _ => false,
    };
    let (hours, name) = if daylight { (4, "EDT") } else { (5, "EST") };
    ((utc - chrono::Duration::hours(hours)).naive_utc(), name)
}

/// `PRETTY_NAME` from /etc/os-release: "Ubuntu 24.04.3 LTS".
pub fn pretty_name(os_release: &str) -> Option<String> {
    os_release
        .lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim().trim_matches('"').to_string())
        .filter(|v| !v.is_empty())
}

fn system_name() -> String {
    let name = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|text| pretty_name(&text));
    match name {
        Some(name) => format!("{name}, {}", std::env::consts::ARCH),
        None => std::env::consts::ARCH.to_string(),
    }
}

fn details(now: DateTime<Utc>) -> Vec<String> {
    let managers: Vec<&str> = managers::available().iter().map(|m| m.as_str()).collect();
    let (time, zone) = toronto(now);
    vec![
        "---".into(),
        format!("- Suisha: {} (command line)", suisha_core::VERSION),
        format!("- OS: {} ({})", setup_file::current_os(), system_name()),
        format!(
            "- Package managers: {}",
            if managers.is_empty() {
                "none".into()
            } else {
                managers.join(", ")
            }
        ),
        format!("- Language: {}", text::language()),
        format!(
            "- Time: {} {zone} (America/Toronto)",
            time.format("%Y-%m-%d %H:%M")
        ),
    ]
}

pub fn encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

pub fn issue_url(headings: &[&str], label: &str, now: DateTime<Utc>) -> String {
    let mut lines: Vec<String> = Vec::new();
    for heading in headings {
        lines.extend([format!("**{heading}**"), String::new(), String::new()]);
    }
    lines.extend(details(now));
    format!(
        "{REPO_URL}/issues/new?labels={label}&body={}",
        encode(&lines.join("\n"))
    )
}

/// Print the link, and open it where a browser is at hand.
fn open(args: &Args, intro: String, url: String) -> Result<i32, String> {
    if args.json {
        print_json(&serde_json::json!({ "url": url }));
        return Ok(0);
    }
    println!("{intro}");
    println!();
    println!("{url}");
    println!();
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let opened = system::has_display()
        && find_bin(opener).is_some_and(|o| run(&o, &[&url], Duration::from_secs(20)).success);
    println!(
        "{}",
        if opened {
            tr!(
                "It was opened in your browser. The version of Suisha and of the system are already filled in.",
                "ブラウザーで開きました。Suisha とシステムの版は、あらかじめ入れてあります。"
            )
        } else {
            tr!(
                "Open it in a browser on any computer. The version of Suisha and of the system are already filled in.",
                "どのコンピューターのブラウザーで開いてもかまいません。Suisha とシステムの版は、あらかじめ入れてあります。"
            )
        }
    );
    Ok(0)
}

pub fn report(args: &Args) -> Result<i32, String> {
    let url = issue_url(
        &[
            "What happened / 何が起きたか",
            "Steps to reproduce / 再現手順",
        ],
        "bug",
        Utc::now(),
    );
    open(
        args,
        tr!(
            "Report a problem with this link (a new issue on GitHub):",
            "次のリンクから問題を報告できます（GitHub の新しい Issue）："
        ),
        url,
    )
}

pub fn request(args: &Args) -> Result<i32, String> {
    let url = issue_url(
        &[
            "What would you like Suisha to do? / Suisha にしてほしいこと",
            "Why would it help? / それがあると何が楽になるか",
        ],
        "enhancement",
        Utc::now(),
    );
    open(
        args,
        tr!(
            "Suggest an improvement with this link (a new issue on GitHub):",
            "次のリンクから要望を伝えられます（GitHub の新しい Issue）："
        ),
        url,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn shown(text: &str) -> String {
        let (time, zone) = toronto(at(text));
        format!("{} {zone}", time.format("%Y-%m-%d %H:%M"))
    }

    #[test]
    fn toronto_time() {
        assert_eq!(shown("2026-11-24T15:30:00Z"), "2026-11-24 10:30 EST");
        assert_eq!(shown("2026-07-01T15:30:00Z"), "2026-07-01 11:30 EDT");
        // 2026: daylight time starts on March 8 and ends on November 1.
        assert_eq!(shown("2026-03-08T06:59:00Z"), "2026-03-08 01:59 EST");
        assert_eq!(shown("2026-03-08T07:00:00Z"), "2026-03-08 03:00 EDT");
        assert_eq!(shown("2026-11-01T05:59:00Z"), "2026-11-01 01:59 EDT");
        assert_eq!(shown("2026-11-01T06:00:00Z"), "2026-11-01 01:00 EST");
        // Midnight UTC is still the day before in Toronto.
        assert_eq!(shown("2027-01-01T02:00:00Z"), "2026-12-31 21:00 EST");
    }

    #[test]
    fn reads_the_system_name() {
        let text = "NAME=\"Ubuntu\"\nPRETTY_NAME=\"Ubuntu 24.04.3 LTS\"\nID=ubuntu\n";
        assert_eq!(pretty_name(text).as_deref(), Some("Ubuntu 24.04.3 LTS"));
        assert_eq!(pretty_name("ID=ubuntu\n"), None);
    }

    #[test]
    fn issue_links() {
        assert_eq!(encode("a b/c?&=日"), "a%20b%2Fc%3F%26%3D%E6%97%A5");
        let url = issue_url(&["What happened"], "bug", at("2026-11-24T15:30:00Z"));
        assert!(
            url.starts_with("https://github.com/moriyama-dev/suisha/issues/new?labels=bug&body=")
        );
        let body = url.split("body=").nth(1).unwrap();
        // Everything after `body=` is one encoded value.
        assert!(!body.contains(['&', ' ', '\n', '#']));
        assert!(body.contains("%2A%2AWhat%20happened%2A%2A"));
        assert!(body.contains("Time%3A%202026-11-24%2010%3A30%20EST"));
        assert!(body.contains("%28command%20line%29"));
    }
}
