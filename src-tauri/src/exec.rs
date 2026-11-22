//! Running external commands without a shell.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{OnceLock, mpsc};
use std::thread;
use std::time::Duration;

/// Directories searched in addition to `PATH`. Apps started from Finder, a
/// desktop launcher or a system timer get a short `PATH`, so snap and
/// Homebrew would be missed.
const EXTRA_DIRS: &[&str] = &[
    // Homebrew on Apple silicon Macs.
    "/opt/homebrew/bin",
    // Homebrew on Intel Macs.
    "/usr/local/bin",
    "/home/linuxbrew/.linuxbrew/bin",
    "/usr/bin",
    "/bin",
    "/usr/sbin",
    "/sbin",
    "/snap/bin",
];

pub fn find_bin(name: &str) -> Option<PathBuf> {
    let from_path = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default();
    from_path
        .into_iter()
        .chain(EXTRA_DIRS.iter().map(PathBuf::from))
        .map(|dir| dir.join(name))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// A `PATH` that contains the usual tool locations, for child processes.
pub fn child_path() -> String {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    for extra in EXTRA_DIRS {
        let extra = PathBuf::from(extra);
        if !dirs.contains(&extra) {
            dirs.push(extra);
        }
    }
    std::env::join_paths(dirs)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| EXTRA_DIRS.join(":"))
}

#[derive(Debug, Clone, Default)]
pub struct Output {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

impl Output {
    /// The last lines of stderr (or stdout), for error messages.
    pub fn tail(&self) -> String {
        let source = if self.stderr.trim().is_empty() {
            &self.stdout
        } else {
            &self.stderr
        };
        let lines: Vec<&str> = source.lines().filter(|l| !l.trim().is_empty()).collect();
        let start = lines.len().saturating_sub(6);
        lines[start..].join("\n")
    }
}

/// A program `sudo -A` runs to ask for the password (macOS, while the window
/// is open). Homebrew uses it for casks whose installer needs administrator
/// rights; without it, those would fail because there is no terminal.
static ASKPASS: OnceLock<PathBuf> = OnceLock::new();

pub fn set_askpass(path: PathBuf) {
    let _ = ASKPASS.set(path);
}

/// The askpass program: shows the standard macOS password dialog through
/// `osascript` and prints what was typed. The prompt is a fixed text, so
/// nothing from outside ends up in the AppleScript.
pub fn askpass_script(prompt: &str) -> String {
    // Inside single quotes (shell) and double quotes (AppleScript): keep
    // the prompt free of both kinds of quote and of backslashes.
    let prompt = prompt
        .replace('\\', "")
        .replace('"', "\u{201d}")
        .replace('\'', "\u{2019}");
    format!(
        "#!/bin/sh\n\
         # Written by Suisha. Asks for the administrator password that Homebrew\n\
         # needs for some apps, using the standard macOS dialog.\n\
         exec /usr/bin/osascript \\\n\
         \x20 -e 'display dialog \"{prompt}\" default answer \"\" with hidden answer with title \"Suisha\" with icon caution' \\\n\
         \x20 -e 'text returned of result'\n"
    )
}

fn base_command(program: &Path, args: &[String]) -> Command {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .env("PATH", child_path())
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("HOMEBREW_NO_AUTO_UPDATE", "1")
        .env("HOMEBREW_NO_ENV_HINTS", "1")
        .env("HOMEBREW_NO_COLOR", "1")
        .env("NONINTERACTIVE", "1")
        .env("DEBIAN_FRONTEND", "noninteractive")
        .stdin(Stdio::null());
    if let Some(askpass) = ASKPASS.get() {
        cmd.env("SUDO_ASKPASS", askpass);
    }
    cmd
}

/// Run a command and collect its output. Used for reading state (lists,
/// searches), so it has a time limit.
pub fn run(program: &Path, args: &[&str], timeout: Duration) -> Output {
    run_with_env(program, args, &[], timeout)
}

/// Like [`run`], with extra environment variables.
pub fn run_with_env(
    program: &Path,
    args: &[&str],
    envs: &[(&str, String)],
    timeout: Duration,
) -> Output {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut cmd = base_command(program, &args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return Output {
                stderr: format!("{}: {e}", program.display()),
                ..Default::default()
            };
        }
    };

    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let out_thread = thread::spawn(move || {
        let mut s = String::new();
        let _ = stdout.read_to_string(&mut s);
        s
    });
    let err_thread = thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });

    let (tx, rx) = mpsc::channel();
    let pid = child.id();
    thread::spawn(move || {
        let status = child.wait();
        let _ = tx.send(status);
    });

    let (status, timed_out) = match rx.recv_timeout(timeout) {
        Ok(status) => (status.ok(), false),
        Err(_) => {
            // Stop the process; its waiting thread then finishes on its own.
            let _ = Command::new("kill").arg(pid.to_string()).status();
            (
                rx.recv_timeout(Duration::from_secs(5))
                    .ok()
                    .and_then(|s| s.ok()),
                true,
            )
        }
    };

    Output {
        success: !timed_out && status.is_some_and(|s| s.success()),
        code: status.and_then(|s| s.code()),
        stdout: out_thread.join().unwrap_or_default(),
        stderr: err_thread.join().unwrap_or_default(),
        timed_out,
    }
}

/// Run a command that changes the system, passing each output line to
/// `on_line` as it appears. No time limit: upgrades can take a long time.
pub fn run_streaming(program: &Path, args: &[String], on_line: &mut dyn FnMut(&str)) -> Output {
    let mut cmd = base_command(program, args);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let message = format!("{}: {e}", program.display());
            on_line(&message);
            return Output {
                stderr: message,
                ..Default::default()
            };
        }
    };

    let (tx, rx) = mpsc::channel::<(bool, String)>();
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let tx_err = tx.clone();
    let t1 = thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = tx.send((false, line));
        }
    });
    let t2 = thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let _ = tx_err.send((true, line));
        }
    });

    let mut out = Output::default();
    for (is_err, line) in rx {
        on_line(&line);
        let target = if is_err {
            &mut out.stderr
        } else {
            &mut out.stdout
        };
        target.push_str(&line);
        target.push('\n');
    }
    let _ = t1.join();
    let _ = t2.join();
    let status = child.wait().ok();
    out.success = status.is_some_and(|s| s.success());
    out.code = status.and_then(|s| s.code());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_common_binaries() {
        assert!(find_bin("sh").is_some());
        assert!(find_bin("surely-not-a-real-binary-name").is_none());
    }

    #[test]
    fn runs_and_times_out() {
        let sh = find_bin("sh").unwrap();
        let ok = run(
            &sh,
            &["-c", "echo hi; echo oops >&2"],
            Duration::from_secs(5),
        );
        assert!(ok.success);
        assert_eq!(ok.stdout.trim(), "hi");
        assert_eq!(ok.stderr.trim(), "oops");

        let slow = run(&sh, &["-c", "sleep 5"], Duration::from_millis(200));
        assert!(slow.timed_out);
        assert!(!slow.success);
    }

    #[test]
    fn askpass_text_is_one_fixed_dialog() {
        let text = askpass_script("Suisha needs \"your\" admin's password\\");
        assert!(text.starts_with("#!/bin/sh\n"));
        assert!(text.contains("display dialog \"Suisha needs \u{201d}your\u{201d} admin\u{2019}s password\" default answer"));
        // Exactly the two quoted -e arguments, nothing else in single quotes.
        assert_eq!(text.matches('\'').count(), 4);
        assert!(text.contains("with hidden answer"));
        assert!(text.ends_with("-e 'text returned of result'\n"));
    }

    #[test]
    fn streams_lines() {
        let sh = find_bin("sh").unwrap();
        let mut seen = Vec::new();
        let out = run_streaming(
            &sh,
            &["-c".into(), "echo one; echo two >&2; exit 3".into()],
            &mut |l| seen.push(l.to_string()),
        );
        assert!(!out.success);
        assert_eq!(out.code, Some(3));
        seen.sort();
        assert_eq!(seen, vec!["one", "two"]);
    }
}
