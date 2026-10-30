mod commands;
mod config;
mod day;
mod images;
mod random;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Write, seal and publish puzzle advent calendars.
#[derive(Parser)]
#[command(name = "advent", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new event repo: config with fresh keys, brand, a first event.
    Init {
        /// Directory to create the repo in; must be empty or not exist.
        dir: PathBuf,
        /// Event id, used as its folder name under events/ (default: current year).
        #[arg(long)]
        event: Option<String>,
    },
    /// Seal a day folder into sealed.json and source.age.
    Seal { day_dir: PathBuf },
    /// Restore a day's plaintext from source.age with the editors key.
    UnsealSource {
        day_dir: PathBuf,
        /// Overwrite plaintext files that differ from the source.
        #[arg(long)]
        force: bool,
    },
    /// Re-encrypt everything to the key lists currently in advent.yaml.
    Reseal {
        event: Option<String>,
        /// Also re-seal every day from its source in the current schema.
        #[arg(long)]
        all: bool,
    },
    /// Certificate commands for the keeper.
    Keeper {
        #[command(subcommand)]
        command: KeeperCommand,
    },
    /// Check that an event repo is complete and its sealed files open.
    Verify {
        event: Option<String>,
        /// Also check every wrapped key with the release key (prints only OK or broken days).
        #[arg(long)]
        release: bool,
    },
}

#[derive(Subcommand)]
enum KeeperCommand {
    /// Seal the certificate payload with the words from the event's sources.
    Seal { event: String },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Init { dir, event } => commands::init::run(&dir, event.as_deref()),
        Command::Seal { day_dir } => commands::seal::run(&day_dir),
        Command::UnsealSource { day_dir, force } => commands::unseal_source::run(&day_dir, force),
        Command::Reseal { event, all } => commands::reseal::run(event.as_deref(), all),
        Command::Keeper {
            command: KeeperCommand::Seal { event },
        } => commands::keeper::run(&event),
        Command::Verify { event, release } => commands::verify::run(event.as_deref(), release),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
