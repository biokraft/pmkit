use clap::{Parser, Subcommand};

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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Doctor) => {
            println!("doctor is not implemented yet");
            Ok(())
        }
        None => {
            println!("run `pmkit --help`");
            Ok(())
        }
    }
}
