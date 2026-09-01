use clap::{Args, Parser, Subcommand};
use pmkit::capabilities::Capabilities;
use pmkit::commands::{home_dir, state_file};
use pmkit::target::Target;

/// Blueprint setup for product managers who work with coding agents.
#[derive(Parser)]
#[command(
    name = "pmkit",
    version,
    about = "Blueprint setup for product managers who work with coding agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Check the tools pmkit relies on and offer to fix what is missing.
    Doctor,
    /// Install, list, refresh or remove the pmkit skills.
    #[command(subcommand)]
    Skill(SkillCmd),
}

#[derive(Subcommand)]
enum SkillCmd {
    /// Write the skill files for one target (or every target, if none is given).
    Install(TargetArg),
    /// Show every tracked file and whether it is current, modified, or missing.
    List,
    /// Re-emit whatever targets are already tracked, without restoring files
    /// that were deliberately deleted.
    Refresh,
    /// Remove the files pmkit wrote for one target (or every target).
    Uninstall(TargetArg),
}

#[derive(Args)]
struct TargetArg {
    /// Which agent to emit for. Omit to act on every target already installed.
    #[arg(long, value_parser = parse_target)]
    target: Option<Target>,
    /// The project to write into. Defaults to the current directory.
    #[arg(long)]
    dir: Option<std::path::PathBuf>,
}

fn parse_target(s: &str) -> Result<Target, String> {
    s.parse::<Target>().map_err(|e| e.to_string())
}

fn run_skill(cmd: SkillCmd) -> anyhow::Result<()> {
    let state = state_file();
    let home = home_dir();
    match cmd {
        SkillCmd::Install(arg) => {
            let dir = arg.dir.unwrap_or(std::env::current_dir()?);
            let targets: Vec<Target> = arg
                .target
                .map(|t| vec![t])
                .unwrap_or_else(|| Target::all().to_vec());
            // Capabilities come from the doctor in `pmkit setup`; a bare
            // `skill install` assumes the best case and lets the preamble be
            // corrected on the next setup run.
            let out = pmkit::commands::skill::install(
                &targets,
                &dir,
                &home,
                &Capabilities::all_present(),
                &state,
            )?;
            for o in out {
                println!("{:<12} {}", o.action.as_str(), o.path.display());
            }
        }
        SkillCmd::List => {
            for (entry, state) in pmkit::commands::skill::list(&state)? {
                println!(
                    "{:<10} {:<12} {}",
                    state.as_str(),
                    entry.target,
                    entry.path.display()
                );
            }
        }
        SkillCmd::Refresh => {
            let dir = std::env::current_dir()?;
            for o in
                pmkit::commands::skill::refresh(&dir, &home, &Capabilities::all_present(), &state)?
            {
                println!("{:<12} {}", o.action.as_str(), o.path.display());
            }
        }
        SkillCmd::Uninstall(arg) => {
            for o in pmkit::commands::skill::remove(arg.target, &state)? {
                println!("{:<12} {}", o.action.as_str(), o.path.display());
            }
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Doctor) => {
            println!("doctor is not implemented yet");
            Ok(())
        }
        Some(Command::Skill(cmd)) => run_skill(cmd),
        None => {
            println!("run `pmkit --help`");
            Ok(())
        }
    }
}
