use crate::capabilities::Capabilities;
use crate::doctor::{probes, runner::RealRunner, table};
use crate::error::Result;
use crate::forge::Forge;
use crate::state::{Action, Outcome};
use crate::target::{destination_for, Destination, Target};
use std::path::Path;

/// What the wizard decided to do, after the human answered.
#[derive(Debug, Clone)]
pub struct Plan {
    pub targets: Vec<Target>,
    pub caps: Capabilities,
}

/// What the human does next on this surface. This is the only place that knows
/// a Cowork bundle has to be uploaded by hand.
///
/// `gate_installed` says whether this target's hook/settings file actually
/// landed on disk this run. For a target whose gates are hook-enforced
/// (`enforces_gates_with_hooks`), claiming enforcement when that file was
/// refused (pmkit found one already there and left it alone) would tell a
/// product manager they are protected when they are not — the one failure
/// mode this whole tool exists to prevent. Targets whose gates are prose
/// only ignore this flag: there is no hook file to have skipped.
pub fn next_steps(target: Target, dest: &Destination, gate_installed: bool) -> String {
    let root = dest.root().display();
    let gate_note = if target.enforces_gates_with_hooks() {
        if gate_installed {
            "The safety gates are enforced here: push, merge and pull-request commands are \
             blocked until you say yes."
                .to_string()
        } else {
            let relpath = target.gate_config_relpath().unwrap_or("its settings file");
            format!(
                "The safety gates are NOT active here: pmkit could not write its {relpath} hook \
                 because you already had one, so push, merge and pull-request commands are not \
                 blocked yet. Merge pmkit's hook into your {relpath} by hand, then run `pmkit \
                 setup` again."
            )
        }
    } else {
        "The safety gates here are written instructions, not enforced automatically. Read them \
         once so you know what your agent has promised."
            .to_string()
    };
    let body = match target {
        Target::ClaudeCode | Target::Cursor | Target::Codex => format!(
            "Your skills are in `{root}` and are ready to use. Start a session and say \
             \"use the pmkit loop\"."
        ),
        Target::Cowork => format!(
            "Your skills are staged in `{root}`. In Cowork, upload each folder under \
             `{root}/skills` as a skill. Then start a chat and say \"use the pmkit loop\"."
        ),
        Target::ChatGpt => format!(
            "Open `{root}/pmkit-chatgpt-instructions.md` and paste all of it into your ChatGPT \
             project's custom instructions."
        ),
    };
    format!("{}\n{}\n{}\n", target.label(), body, gate_note)
}

/// Whether this target's hook/settings file actually landed on disk this
/// run, per the `Vec<Outcome>` from `commands::skill::install`. Targets that
/// don't enforce gates with hooks have no such file to check, so they
/// trivially report `true` — `next_steps` never consults the flag for them.
fn gate_installed(outcomes: &[Outcome], target: Target) -> bool {
    let Some(relpath) = target.gate_config_relpath() else {
        return true;
    };
    outcomes.iter().any(|o| {
        o.target == target.as_str()
            && o.path.ends_with(relpath)
            && matches!(
                o.action,
                Action::Installed | Action::Refreshed | Action::Unchanged
            )
    })
}

/// Non-interactive setup: probe, emit, explain. `pmkit setup --yes` and the
/// tests use this; the interactive path adds the questions around it.
pub fn run_unattended(
    targets: &[Target],
    project_dir: &Path,
    home: &Path,
    state_file: &Path,
) -> Result<()> {
    let probes = probes::run_all(&RealRunner, home, Forge::GitHub);
    println!("{}", table(&probes));
    let caps = probes::capabilities_from(&probes, Forge::GitHub);

    let outcomes = crate::commands::skill::install(targets, project_dir, home, &caps, state_file)?;
    println!();
    for o in &outcomes {
        println!("{:<24} {}", o.action.as_str(), o.path.display());
    }

    // A skipped file is the one outcome a product manager must not scroll past.
    // It means they already had a file of that name — their own AGENTS.md, their
    // own settings.json — so pmkit refused to overwrite it. The consequence is
    // that whatever that file was meant to carry is NOT installed: on a Claude
    // Code or Cursor project, that can mean the gates are not enforced at all,
    // while the emitted preamble still says they are. Silence here would make
    // pmkit a liar, so it gets its own block, not a row in a list.
    let skipped: Vec<&Outcome> = outcomes
        .iter()
        .filter(|o| o.action == Action::SkippedModified)
        .collect();
    if !skipped.is_empty() {
        println!(
            "\n{} file(s) already existed and were left alone:\n",
            skipped.len()
        );
        for o in &skipped {
            println!("  {}", o.path.display());
        }
        println!(
            "\npmkit did not overwrite these, so what they were meant to contain is not installed.\n\
             Open each one and merge in what you want by hand, or move it aside and run\n\
             `pmkit setup` again. If one of them is a settings.json on Claude Code or Cursor, the\n\
             safety gates are NOT enforced until you do."
        );
    }
    // A failed write is at least as serious as a refused overwrite: the file
    // simply never landed, so whatever it carried — for a hook-enforced
    // target, that can be the gate config itself — is not installed. Give it
    // its own block for the same reason `skipped` gets one, rather than
    // letting it scroll past as a single row in the list above.
    let failed: Vec<&Outcome> = outcomes
        .iter()
        .filter(|o| o.action == Action::Failed)
        .collect();
    if !failed.is_empty() {
        println!("\n{} file(s) could not be written:\n", failed.len());
        for o in &failed {
            println!("  {}", o.path.display());
        }
        println!(
            "\npmkit could not write these, so whatever they were meant to contain is not \
             installed. Check the path is writable and run `pmkit setup` again. If one of them \
             is a hook-enforced target's gate config (settings.json on Claude Code, hooks.json on \
             Cursor), the safety gates are NOT active until it lands."
        );
    }
    println!("\nWhat to do next\n");
    for &t in targets {
        let installed = gate_installed(&outcomes, t);
        println!(
            "{}",
            next_steps(t, &destination_for(t, project_dir, home), installed)
        );
    }
    if !caps.superpowers {
        println!(
            "One thing is missing: Superpowers. The build stage needs it. Install it in your \
             agent, then run `pmkit setup` again so the skills stop warning about it."
        );
    }
    Ok(())
}

/// Interactive setup. Asks which agents the human uses, shows the doctor's
/// table, asks before offering any fix, then emits.
pub fn run(
    project_dir: &Path,
    home: &Path,
    state_file: &Path,
    preselected: Option<Target>,
) -> Result<()> {
    let targets: Vec<Target> = match preselected {
        Some(t) => vec![t],
        None => {
            let options: Vec<&'static str> = Target::all().iter().map(|t| t.label()).collect();
            let chosen = inquire::MultiSelect::new("Which agents do you use?", options.clone())
                .prompt()
                .unwrap_or_default();
            Target::all()
                .into_iter()
                .filter(|t| chosen.contains(&t.label()))
                .collect()
        }
    };
    if targets.is_empty() {
        println!("Nothing selected, so nothing was written.");
        return Ok(());
    }
    run_unattended(&targets, project_dir, home, state_file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::{destination_for, Target};
    use std::path::Path;

    #[test]
    fn in_repo_targets_are_described_as_ready_to_use() {
        let dest = destination_for(Target::Cursor, Path::new("/p"), Path::new("/h"));
        let text = next_steps(Target::Cursor, &dest, true);
        assert!(text.contains("ready to use"));
        assert!(text.contains("/p"));
    }

    #[test]
    fn cowork_is_told_to_upload_and_where_from() {
        let dest = destination_for(Target::Cowork, Path::new("/p"), Path::new("/h"));
        let text = next_steps(Target::Cowork, &dest, true);
        assert!(text.contains("upload"));
        assert!(text.contains("/h/pmkit-cowork"));
    }

    #[test]
    fn chatgpt_is_told_to_paste_and_names_the_file() {
        let dest = destination_for(Target::ChatGpt, Path::new("/p"), Path::new("/h"));
        let text = next_steps(Target::ChatGpt, &dest, true);
        assert!(text.contains("paste"));
        assert!(text.contains("pmkit-chatgpt-instructions.md"));
    }

    #[test]
    fn prose_only_targets_get_told_the_gates_are_not_enforced() {
        for t in [Target::Cowork, Target::ChatGpt, Target::Codex] {
            let dest = destination_for(t, Path::new("/p"), Path::new("/h"));
            assert!(
                next_steps(t, &dest, true).contains("not enforced automatically"),
                "{}",
                t.as_str()
            );
        }
    }

    #[test]
    fn every_target_produces_next_steps() {
        for t in Target::all() {
            let dest = destination_for(t, Path::new("/p"), Path::new("/h"));
            assert!(
                !next_steps(t, &dest, true).trim().is_empty(),
                "{}",
                t.as_str()
            );
        }
    }

    /// The load-bearing case from fix round 1: when the settings.json that
    /// carries a hook-enforced target's gates was refused (the human already
    /// had one), the closing "next steps" text must NOT claim the gates are
    /// enforced. Before `next_steps` took `gate_installed`, this target's
    /// branch was unconditional and always printed the "enforced" sentence
    /// regardless of what actually landed on disk — this is what pins that
    /// it no longer does.
    #[test]
    fn a_hook_enforced_target_with_a_refused_settings_file_does_not_claim_enforcement() {
        let dest = destination_for(Target::ClaudeCode, Path::new("/p"), Path::new("/h"));
        let text = next_steps(Target::ClaudeCode, &dest, false);
        assert!(
            !text.contains("The safety gates are enforced here"),
            "{text}"
        );
        assert!(text.contains("not") && text.contains("active"), "{text}");
    }

    #[test]
    fn a_cursor_refusal_names_the_cursor_hooks_file_not_settings_json() {
        let dest = destination_for(Target::Cursor, Path::new("/p"), Path::new("/h"));
        let text = next_steps(Target::Cursor, &dest, false);
        assert!(text.contains(".cursor/hooks.json"), "{text}");
        assert!(!text.contains("settings.json"), "{text}");
    }

    #[test]
    fn gate_installed_is_false_when_the_settings_file_was_skipped() {
        let outcomes = vec![Outcome {
            path: std::path::PathBuf::from("/p/.claude/settings.json"),
            target: Target::ClaudeCode.as_str().to_string(),
            action: Action::SkippedModified,
        }];
        assert!(!gate_installed(&outcomes, Target::ClaudeCode));
    }

    #[test]
    fn gate_installed_is_true_when_the_settings_file_landed() {
        let outcomes = vec![Outcome {
            path: std::path::PathBuf::from("/p/.claude/settings.json"),
            target: Target::ClaudeCode.as_str().to_string(),
            action: Action::Installed,
        }];
        assert!(gate_installed(&outcomes, Target::ClaudeCode));
    }

    /// The bug this task fixes: `gate_installed` used to look for a literal
    /// `settings.json` outcome, which Cursor never produces, so a perfectly
    /// successful Cursor setup — its `hooks.json` installed — was reported
    /// as having its safety gates NOT active. With `gate_config_relpath`
    /// asking the right question per target, a successful Cursor install
    /// must claim enforcement.
    #[test]
    fn a_successful_cursor_setup_claims_enforcement() {
        let outcomes = vec![Outcome {
            path: std::path::PathBuf::from("/p/.cursor/hooks.json"),
            target: Target::Cursor.as_str().to_string(),
            action: Action::Installed,
        }];
        assert!(gate_installed(&outcomes, Target::Cursor));
        let dest = destination_for(Target::Cursor, Path::new("/p"), Path::new("/h"));
        let text = next_steps(
            Target::Cursor,
            &dest,
            gate_installed(&outcomes, Target::Cursor),
        );
        assert!(
            text.contains("The safety gates are enforced here"),
            "{text}"
        );
    }
}
