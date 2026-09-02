use crate::capabilities::Capabilities;
use crate::doctor::{probes, runner::RealRunner, table};
use crate::error::Result;
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
pub fn next_steps(target: Target, dest: &Destination) -> String {
    let root = dest.root().display();
    let gate_note = if target.enforces_gates_with_hooks() {
        "The safety gates are enforced here: push, merge and pull-request commands are blocked \
         until you say yes."
            .to_string()
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

/// Non-interactive setup: probe, emit, explain. `pmkit setup --yes` and the
/// tests use this; the interactive path adds the questions around it.
pub fn run_unattended(
    targets: &[Target],
    project_dir: &Path,
    home: &Path,
    state_file: &Path,
) -> Result<()> {
    let probes = probes::run_all(&RealRunner, home);
    println!("{}", table(&probes));
    let caps = probes::capabilities_from(&probes);

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
    println!("\nWhat to do next\n");
    for &t in targets {
        println!("{}", next_steps(t, &destination_for(t, project_dir, home)));
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
pub fn run(project_dir: &Path, home: &Path, state_file: &Path) -> Result<()> {
    let options: Vec<&'static str> = Target::all().iter().map(|t| t.label()).collect();
    let chosen = inquire::MultiSelect::new("Which agents do you use?", options.clone())
        .prompt()
        .unwrap_or_default();
    let targets: Vec<Target> = Target::all()
        .into_iter()
        .filter(|t| chosen.contains(&t.label()))
        .collect();
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
        let text = next_steps(Target::Cursor, &dest);
        assert!(text.contains("ready to use"));
        assert!(text.contains("/p"));
    }

    #[test]
    fn cowork_is_told_to_upload_and_where_from() {
        let dest = destination_for(Target::Cowork, Path::new("/p"), Path::new("/h"));
        let text = next_steps(Target::Cowork, &dest);
        assert!(text.contains("upload"));
        assert!(text.contains("/h/pmkit-cowork"));
    }

    #[test]
    fn chatgpt_is_told_to_paste_and_names_the_file() {
        let dest = destination_for(Target::ChatGpt, Path::new("/p"), Path::new("/h"));
        let text = next_steps(Target::ChatGpt, &dest);
        assert!(text.contains("paste"));
        assert!(text.contains("pmkit-chatgpt-instructions.md"));
    }

    #[test]
    fn prose_only_targets_get_told_the_gates_are_not_enforced() {
        for t in [Target::Cowork, Target::ChatGpt, Target::Codex] {
            let dest = destination_for(t, Path::new("/p"), Path::new("/h"));
            assert!(
                next_steps(t, &dest).contains("not enforced automatically"),
                "{}",
                t.as_str()
            );
        }
    }

    #[test]
    fn every_target_produces_next_steps() {
        for t in Target::all() {
            let dest = destination_for(t, Path::new("/p"), Path::new("/h"));
            assert!(!next_steps(t, &dest).trim().is_empty(), "{}", t.as_str());
        }
    }
}
