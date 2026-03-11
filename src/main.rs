mod cli;
mod config;
mod error;
mod sync;
mod time;
mod tui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Sync(args)) => {
            sync::run(args).await?;
        }
        Some(Commands::Time(args)) => {
            time::run(args).await?;
        }
        Some(Commands::Settings) => {
            tui::run_settings().await?;
        }
        None => {
            tui::run_interactive().await?;
        }
    }

    Ok(())
}
