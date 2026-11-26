//! Reading the command line.

use suisha_core::model::ManagerId;

use crate::text::tr;

/// The commands `suisha` understands.
pub const COMMANDS: [&str; 19] = [
    "status",
    "updates",
    "update",
    "list",
    "search",
    "install",
    "remove",
    "history",
    "schedule",
    "on",
    "off",
    "export",
    "setup",
    "cleanup",
    "self-update",
    "report",
    "request",
    "help",
    "version",
];

#[derive(Debug, Default, PartialEq)]
pub struct Args {
    pub command: String,
    pub positional: Vec<String>,
    /// Print the result as JSON on standard output.
    pub json: bool,
    /// Do not ask before changing anything.
    pub yes: bool,
    /// Show the commands without running them.
    pub dry_run: bool,
    /// Only these package managers. Empty means all that are installed.
    pub managers: Vec<ManagerId>,
    pub limit: Option<usize>,
    pub minute: Option<u8>,
    /// `--linger` / `--no-linger`.
    pub linger: Option<bool>,
    /// `--auto on|off`.
    pub auto: Option<bool>,
    pub help: bool,
}

pub fn manager_from(name: &str) -> Option<ManagerId> {
    match name {
        "cask" => Some(ManagerId::BrewCask),
        _ => ManagerId::ALL.into_iter().find(|m| m.as_str() == name),
    }
}

fn manager_list(value: &str) -> Result<Vec<ManagerId>, String> {
    value
        .split(',')
        .filter(|name| !name.is_empty())
        .map(|name| {
            manager_from(name).ok_or_else(|| {
                tr!(
                    "{name} is not a package manager Suisha knows (apt, snap, flatpak, brew, brew-cask, mas).",
                    "{name} は Suisha が扱うパッケージマネージャーではありません（apt、snap、flatpak、brew、brew-cask、mas）。"
                )
            })
        })
        .collect()
}

fn on_off(value: &str) -> Result<bool, String> {
    match value {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(tr!(
            "--auto takes on or off.",
            "--auto には on か off を指定してください。"
        )),
    }
}

pub fn parse(args: &[String]) -> Result<Args, String> {
    let mut parsed = Args::default();
    let mut words = Vec::new();
    let mut options_ended = false;
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        if options_ended || !arg.starts_with('-') || arg == "-" {
            words.push(arg.clone());
            continue;
        }
        // `--name=value` and `--name value` are the same.
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_string())),
            _ => (arg.as_str(), None),
        };
        let mut value = || {
            inline
                .clone()
                .or_else(|| iter.next().cloned())
                .ok_or_else(|| tr!("{name} needs a value.", "{name} には値が必要です。"))
        };
        match name {
            "--" => options_ended = true,
            "--json" => parsed.json = true,
            "-y" | "--yes" => parsed.yes = true,
            "--dry-run" => parsed.dry_run = true,
            "-h" | "--help" => parsed.help = true,
            "--linger" => parsed.linger = Some(true),
            "--no-linger" => parsed.linger = Some(false),
            "-m" | "--manager" | "--managers" => {
                for m in manager_list(&value()?)? {
                    if !parsed.managers.contains(&m) {
                        parsed.managers.push(m);
                    }
                }
            }
            "-n" | "--limit" => {
                parsed.limit = Some(value()?.parse().map_err(|_| {
                    tr!(
                        "--limit takes a number.",
                        "--limit には数を指定してください。"
                    )
                })?);
            }
            "--minute" => {
                parsed.minute =
                    Some(value()?.parse().ok().filter(|m| *m <= 59).ok_or_else(|| {
                        tr!(
                            "--minute takes a number from 0 to 59.",
                            "--minute には 0〜59 の数を指定してください。"
                        )
                    })?);
            }
            "--auto" => parsed.auto = Some(on_off(&value()?)?),
            _ => {
                return Err(tr!(
                    "Unknown option: {name}",
                    "知らないオプションです：{name}"
                ));
            }
        }
    }

    let mut words = words.into_iter();
    parsed.command = words.next().unwrap_or_else(|| "help".into());
    parsed.positional = words.collect();
    if !COMMANDS.contains(&parsed.command.as_str()) {
        let command = &parsed.command;
        return Err(tr!(
            "Unknown command: {command}",
            "知らないコマンドです：{command}"
        ));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_words(words: &[&str]) -> Result<Args, String> {
        parse(&words.iter().map(|w| w.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn options_can_be_anywhere() {
        let a = parse_words(&["--json", "update", "-y", "apt:git", "-m", "apt,snap"]).unwrap();
        assert_eq!(a.command, "update");
        assert_eq!(a.positional, vec!["apt:git"]);
        assert!(a.json && a.yes && !a.dry_run);
        assert_eq!(a.managers, vec![ManagerId::Apt, ManagerId::Snap]);
    }

    #[test]
    fn values_with_and_without_equals() {
        let a = parse_words(&["history", "--limit=5"]).unwrap();
        assert_eq!(a.limit, Some(5));
        let a = parse_words(&["history", "-n", "7"]).unwrap();
        assert_eq!(a.limit, Some(7));
        assert!(parse_words(&["history", "-n"]).is_err());
        assert!(parse_words(&["history", "-n", "many"]).is_err());
    }

    #[test]
    fn schedule_words_stay_in_order() {
        let a = parse_words(&["schedule", "set", "weekly", "mon", "03:30", "--no-linger"]).unwrap();
        assert_eq!(a.positional, vec!["set", "weekly", "mon", "03:30"]);
        assert_eq!(a.linger, Some(false));
        let a = parse_words(&["schedule", "set", "every", "6h", "--minute", "15"]).unwrap();
        assert_eq!(a.minute, Some(15));
        assert!(parse_words(&["schedule", "set", "hourly", "--minute", "60"]).is_err());
    }

    #[test]
    fn on_and_off_are_commands() {
        let a = parse_words(&["on", "-m", "apt", "--minute", "30"]).unwrap();
        assert_eq!(a.command, "on");
        assert_eq!(a.managers, vec![ManagerId::Apt]);
        assert_eq!(a.minute, Some(30));
        assert_eq!(parse_words(&["off"]).unwrap().command, "off");
    }

    #[test]
    fn rejects_what_it_does_not_know() {
        assert!(parse_words(&["upgrade"]).is_err());
        assert!(parse_words(&["update", "--force"]).is_err());
        assert!(parse_words(&["list", "-m", "pacman"]).is_err());
        assert!(parse_words(&["self-update", "--auto", "maybe"]).is_err());
    }

    #[test]
    fn two_dashes_end_the_options() {
        let a = parse_words(&["search", "--", "--help"]).unwrap();
        assert_eq!(a.positional, vec!["--help"]);
        assert!(!a.help);
        assert_eq!(parse_words(&[]).unwrap().command, "help");
        assert_eq!(parse_words(&["cask"]).ok(), None);
        assert_eq!(manager_from("cask"), Some(ManagerId::BrewCask));
    }
}
