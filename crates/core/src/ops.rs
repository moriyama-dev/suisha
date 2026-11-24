//! Running planned steps and recording them in the history.

use serde::Serialize;

use crate::exec::run_streaming;
use crate::managers::Step;
use crate::model::ManagerId;
use crate::store::{self, HistoryEntry};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepOutcome {
    pub managers: Vec<ManagerId>,
    pub packages: Vec<String>,
    pub success: bool,
    /// The password dialog was dismissed.
    pub cancelled: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpResult {
    pub success: bool,
    pub cancelled: bool,
    pub steps: Vec<StepOutcome>,
}

/// How a step is shown in the log: the program name and its arguments.
pub fn display_command(step: &Step) -> String {
    let name = step
        .program
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut parts = vec![name];
    parts.extend(step.args.iter().map(|a| {
        // Show helpers by their name, not their full path.
        if a.starts_with('/') {
            a.rsplit('/').next().unwrap_or(a).to_string()
        } else {
            a.clone()
        }
    }));
    parts.join(" ")
}

pub fn execute(
    steps: &[Step],
    trigger: &str,
    action: &str,
    on_line: &mut dyn FnMut(&str),
) -> OpResult {
    let mut outcomes = Vec::new();
    let mut cancelled_any = false;
    for step in steps {
        if cancelled_any {
            break;
        }
        on_line(&format!("$ {}", display_command(step)));
        let out = run_streaming(&step.program, &step.args, on_line);
        let is_pkexec = step.program.file_name().is_some_and(|n| n == "pkexec");
        let cancelled = is_pkexec && matches!(out.code, Some(126));
        cancelled_any |= cancelled;
        outcomes.push(StepOutcome {
            managers: step.managers.clone(),
            packages: step.packages.clone(),
            success: out.success,
            cancelled,
            message: if out.success {
                String::new()
            } else if cancelled {
                "cancelled".into()
            } else {
                out.tail()
            },
        });
    }

    if action != "refresh" {
        let time = store::now();
        let entries: Vec<HistoryEntry> = outcomes
            .iter()
            .map(|o| HistoryEntry {
                time: time.clone(),
                trigger: trigger.into(),
                action: action.into(),
                managers: o.managers.clone(),
                packages: o.packages.clone(),
                success: o.success,
                message: o.message.clone(),
            })
            .collect();
        store::append_history(&entries);
    }

    OpResult {
        success: outcomes.iter().all(|o| o.success),
        cancelled: cancelled_any,
        steps: outcomes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn shows_helper_names() {
        let step = Step {
            managers: vec![ManagerId::Apt],
            program: PathBuf::from("/usr/bin/pkexec"),
            args: vec![
                "/usr/lib/suisha/suisha-root".into(),
                "--apt-install".into(),
                "git".into(),
            ],
            packages: vec!["git".into()],
        };
        assert_eq!(
            display_command(&step),
            "pkexec suisha-root --apt-install git"
        );
    }
}
