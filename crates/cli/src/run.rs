//! Showing, confirming and running the commands that change the system.

use std::io::{IsTerminal, Write};
use std::process::Command;

use serde::Serialize;
use suisha_core::managers::Step;
use suisha_core::ops::{self, OpResult};
use suisha_core::privileged::{self, Elevation};

use crate::args::Args;
use crate::text::tr;

/// Print something meant for people. With `--json`, standard output carries
/// only the JSON, so this goes to standard error.
pub fn say(args: &Args, text: impl AsRef<str>) {
    if args.json {
        eprintln!("{}", text.as_ref());
    } else {
        println!("{}", text.as_ref());
    }
}

pub fn warn(text: impl AsRef<str>) {
    eprintln!("suisha: {}", text.as_ref());
}

pub fn print_json<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".into())
    );
}

pub fn report_hint() -> String {
    tr!(
        "If this looks like a problem in Suisha, `suisha report` opens a report for it.",
        "Suisha の不具合と思われるときは、`suisha report` で報告できます。"
    )
}

/// Ask before changing anything. `--yes` answers for the person; without a
/// terminal nobody can answer, so that is an error instead of a silent "no".
pub fn confirm(args: &Args, question: &str) -> Result<bool, String> {
    if args.yes {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        return Err(tr!(
            "Nobody is there to confirm this. Add --yes to run it without being asked.",
            "確認に答える人がいません。確認なしで実行するには --yes を付けてください。"
        ));
    }
    eprint!("{question} [y/N] ");
    let _ = std::io::stderr().flush();
    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .map_err(|e| e.to_string())?;
    Ok(matches!(answer.trim().to_lowercase().as_str(), "y" | "yes"))
}

/// Let `sudo` ask for the password now, before any output scrolls by. The
/// commands that follow reuse it.
pub fn sudo_login() -> Result<(), String> {
    let Elevation::Sudo(sudo) = privileged::interactive()? else {
        return Ok(());
    };
    let accepted = Command::new(sudo)
        .arg("-v")
        .status()
        .map_err(|e| e.to_string())?
        .success();
    if accepted {
        Ok(())
    } else {
        Err(tr!(
            "Administrator rights are needed, and sudo did not grant them.",
            "管理者権限が必要ですが、sudo で得られませんでした。"
        ))
    }
}

fn uses_sudo(steps: &[Step]) -> bool {
    steps
        .iter()
        .any(|s| s.program.file_name().is_some_and(|n| n == "sudo"))
}

/// Show the commands, then ask. `Ok(false)` means they should not be run
/// (`--dry-run`, or the answer was no).
pub fn prepare(args: &Args, steps: &[Step]) -> Result<bool, String> {
    say(
        args,
        tr!("These commands will run:", "次のコマンドを実行します："),
    );
    for step in steps {
        say(args, format!("  $ {}", ops::display_command(step)));
    }
    if args.dry_run {
        if args.json {
            let commands: Vec<String> = steps.iter().map(ops::display_command).collect();
            print_json(&serde_json::json!({ "dryRun": true, "commands": commands }));
        }
        return Ok(false);
    }
    if !confirm(args, &tr!("Continue?", "実行しますか？"))? {
        say(args, tr!("Nothing was changed.", "何も変更していません。"));
        return Ok(false);
    }
    if uses_sudo(steps) {
        sudo_login()?;
    }
    Ok(true)
}

/// Run the commands, showing their output as it appears, and write the
/// history.
pub fn perform(args: &Args, steps: &[Step], action: &str) -> OpResult {
    ops::execute(steps, "manual", action, &mut |line| say(args, line))
}

/// Say how it went. Returns the exit code.
pub fn finish(args: &Args, result: &OpResult) -> i32 {
    if args.json {
        print_json(result);
    }
    if result.success {
        say(args, tr!("Done.", "完了しました。"));
        return 0;
    }
    for step in result.steps.iter().filter(|s| !s.success) {
        let managers: Vec<&str> = step.managers.iter().map(|m| m.as_str()).collect();
        let managers = managers.join(", ");
        warn(tr!("{managers} failed:", "{managers} が失敗しました："));
        for line in crate::text::message(&step.message).lines() {
            eprintln!("  {line}");
        }
    }
    eprintln!("{}", report_hint());
    1
}

/// The usual case: show, ask, run, report.
pub fn execute(args: &Args, steps: &[Step], action: &str) -> Result<i32, String> {
    if steps.is_empty() {
        say(args, tr!("Nothing to do.", "することはありません。"));
        return Ok(0);
    }
    if !prepare(args, steps)? {
        // Declining is not a success, a dry run is.
        return Ok(if args.dry_run { 0 } else { 1 });
    }
    let result = perform(args, steps, action);
    Ok(finish(args, &result))
}
