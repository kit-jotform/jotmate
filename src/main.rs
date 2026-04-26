use anyhow::Result;
use clap::Parser;
use jotmate::cli::{Cli, Commands};
use jotmate::ctx::Ctx;
use jotmate::{sync, time, tui, update};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let ctx = Ctx::production();

    match cli.command {
        Some(Commands::Sync(args)) => {
            sync::run(&ctx, args).await?;
        }
        Some(Commands::Time(args)) => {
            time::run(&ctx, args).await?;
        }
        Some(Commands::Settings) => {
            tui::run_settings(ctx).await?;
        }
        Some(Commands::Update) => {
            update::run().await?;
        }
        None => {
            tui::run_interactive(ctx).await?;
        }
    }

    Ok(())
}
