//! The `suisha` command: everything the window does, from a terminal.
//!
//! The window's program links this too, so that `suisha --auto-update` and
//! the other commands also work from the app on macOS.

mod args;
mod feedback;
mod own;
mod packages;
mod run;
mod schedule;
mod setup;
mod status;
mod text;

use std::path::PathBuf;

use suisha_core::{background, privileged, system};

use text::tr;

/// What timers and release checks call; these never ask anything.
const BACKGROUND: [&str; 3] = ["--auto-update", "--self-update", "--self-check"];

/// Whether `arg`, as the first argument, is something this module handles.
/// The window's program uses it to decide between a command and the window.
pub fn is_command(arg: &str) -> bool {
    args::COMMANDS.contains(&arg)
        || BACKGROUND.contains(&arg)
        || matches!(arg, "--version" | "-V" | "--help" | "-h")
}

/// The window's program, when it is installed next to this command and a
/// desktop is there to show it.
pub fn window_program() -> Option<PathBuf> {
    if !cfg!(target_os = "linux") || !system::has_display() {
        return None;
    }
    let window = std::env::current_exe().ok()?.with_file_name("suisha-gui");
    window.is_file().then_some(window)
}

fn help() -> String {
    let version = suisha_core::VERSION;
    let issues = format!("{}/issues", feedback::REPO_URL);
    tr!(
        "Suisha {version}: keep this computer's software up to date.
apt, snap, Flatpak and Homebrew with one command.

Usage: suisha <command> [options]

See
  status                 Waiting updates, the schedule, and what the system asks for
  updates                List the updates that are waiting
  list                   List what is installed
  search NAME            Search every package manager
  history                What Suisha changed, newest first (-n COUNT)

Change
  update                 Install every update
  update PACKAGE...      Update only these
  install PACKAGE...     For example: suisha install apt:htop snap:btop
  remove PACKAGE...      Remove
  cleanup                Remove packages nothing needs any more (apt autoremove)

Automate
  schedule               Show the automatic updates
  schedule set daily 03:00
  schedule set weekly sun 03:00
  schedule set every 6h  1h, 3h, 6h or 12h; --minute 30 for half past
  schedule on | off
  export [FILE]          Save the list of installed apps (prints it without FILE)
  setup FILE             Install what a saved list has and this computer lacks

Suisha itself
  self-update            Update Suisha now (--auto on|off: once a day, by itself)
  report                 Report a problem
  request                Suggest an improvement
  version, help

Options
  -m, --manager NAME     Only this package manager: apt, snap, flatpak, brew
  -y, --yes              Do not ask before changing anything
      --dry-run          Show the commands without running them
      --json             Print the result as JSON
      --linger, --no-linger
                         (schedule) keep the timer running while logged out, or not

A package is written MANAGER:NAME, the way `suisha list` and `suisha search` print it.
Every command Suisha runs is shown before it runs. Suisha sends nothing about you or
this computer anywhere.

Problems and requests: {issues}",
        "Suisha {version}：このコンピューターのソフトウェアを最新に保ちます。
apt・snap・Flatpak・Homebrew を1つのコマンドで扱います。

使い方：suisha <コマンド> [オプション]

見る
  status                 更新の有無、自動更新の予定、システムからのお知らせ
  updates                更新があるものの一覧
  list                   入っているものの一覧
  search 名前            すべてのパッケージマネージャーから探す
  history                Suisha が変更したことの記録（新しい順。-n 件数）

変更する
  update                 すべての更新を入れる
  update パッケージ...   指定したものだけ更新する
  install パッケージ...  例：suisha install apt:htop snap:btop
  remove パッケージ...   削除する
  cleanup                不要になったパッケージを削除する（apt autoremove）

自動化
  schedule               自動更新の設定を表示する
  schedule set daily 03:00
  schedule set weekly sun 03:00
  schedule set every 6h  1h・3h・6h・12h。--minute 30 で毎時 30 分
  schedule on | off
  export [ファイル]      入っているアプリの一覧を保存する（ファイルを省くと画面に出す）
  setup ファイル         保存した一覧のうち、このコンピューターに無いものを入れる

Suisha 自体
  self-update            Suisha を今すぐ更新する（--auto on|off：1日1回、自動で）
  report                 問題を報告
  request                要望を伝える
  version, help

オプション
  -m, --manager 名前     このパッケージマネージャーだけ：apt、snap、flatpak、brew
  -y, --yes              変更の前に確認しない
      --dry-run          コマンドを表示するだけで、実行しない
      --json             結果を JSON で出す
      --linger, --no-linger
                         （schedule）ログアウト中もタイマーを動かす／動かさない

パッケージは「マネージャー:名前」の形で書きます。`suisha list` と `suisha search` の表示と同じです。
Suisha が実行するコマンドは、実行の前にすべて表示します。あなたやこのコンピューターの情報を、
どこにも送りません。

問題の報告と要望：{issues}"
    )
}

/// `suisha list | head` closes the pipe while the list is still being
/// printed. Rust ignores SIGPIPE by default, which turns that into an error
/// message; the usual behaviour of a command line program is to end quietly.
fn quit_quietly_on_closed_pipe() {
    // SAFETY: sets a signal back to its default action, before any other
    // thread exists and without installing a handler.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

/// Run one command. Returns the exit code: 0 when it worked, 1 when it did
/// not, 2 when the command line could not be understood.
pub fn run(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("--auto-update") => return background::auto_update(),
        Some("--self-update") => return background::self_update(),
        Some("--self-check") => return background::self_check(),
        _ => {}
    }
    // From here on a person is at a terminal: passwords go through sudo.
    privileged::use_terminal();
    quit_quietly_on_closed_pipe();

    let args = match args.first().map(String::as_str) {
        Some("--version" | "-V") => vec!["version".to_string()],
        _ => args.to_vec(),
    };
    let args = match args::parse(&args) {
        Ok(args) => args,
        Err(e) => {
            run::warn(e);
            eprintln!(
                "{}",
                tr!(
                    "`suisha help` lists the commands.",
                    "`suisha help` でコマンドの一覧を見られます。"
                )
            );
            return 2;
        }
    };
    if args.help || args.command == "help" {
        println!("{}", help());
        return 0;
    }
    let result = match args.command.as_str() {
        "version" => {
            println!("suisha {}", suisha_core::VERSION);
            Ok(0)
        }
        "status" => status::status(&args),
        "updates" => packages::updates(&args),
        "update" => packages::update(&args),
        "list" => packages::list(&args),
        "search" => packages::search(&args),
        "install" => packages::install(&args),
        "remove" => packages::remove(&args),
        "cleanup" => packages::cleanup(&args),
        "history" => status::history(&args),
        "schedule" => schedule::schedule(&args),
        "export" => setup::export(&args),
        "setup" => setup::setup(&args),
        "self-update" => own::self_update(&args),
        "report" => feedback::report(&args),
        "request" => feedback::request(&args),
        other => unreachable!("{other} passed the command check"),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            run::warn(e);
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_commands_from_what_the_desktop_passes() {
        for command in [
            "update",
            "status",
            "--auto-update",
            "--self-check",
            "--version",
            "-h",
        ] {
            assert!(is_command(command), "{command}");
        }
        // A file or a launcher argument opens the window instead.
        for other in ["", "-psn_0_12345", "/home/me/list.json", "--no-sandbox"] {
            assert!(!is_command(other), "{other}");
        }
    }

    #[test]
    fn help_names_every_command() {
        let text = help();
        for command in args::COMMANDS {
            assert!(text.contains(command), "{command}");
        }
    }
}
