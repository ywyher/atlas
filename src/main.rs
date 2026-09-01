pub mod consts {
    pub const DATA_FOLDER: &str = "data";
}

mod anilist;
mod config;
mod jiten;
mod merge;
mod meilisearch;

use std::path::PathBuf;

use crate::anilist::client::Client as AnilistClient;
use crate::jiten::client::Client as JitenClient;
use crate::merge::merge::Merge;
use crate::meilisearch::client::Client as MeiliClient;
use crate::config::Config;
use anyhow::Result;
use tracing_subscriber::{self, EnvFilter};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    // global -> allows args after commands
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    debug: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Full,
    Incremental
}


#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let tracing_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(match cli.debug {
            1 => "atlas=debug",
            _ => "atlas=info"
        }));
    tracing_subscriber::fmt().with_env_filter(tracing_filter).init();

    let config = Config::from_env()?;

    let anilist = AnilistClient::new(&config);
    let mode = cli.command.unwrap_or(Commands::Full);
    let has_changes = match mode {
        Commands::Full => {
            anilist.scrape().await?;
            true
        }
        Commands::Incremental => {
            let res = anilist.scrape_incremental().await?;
            let changed = res.real_changes > 0 || res.new_entries > 0;
            if !changed {
                tracing::info!(
                    real_changes = res.real_changes,
                    new_entries = res.new_entries,
                    noise = res.noise,
                    "no anilist changes detected, skipping jiten/merge/meili"
                );
            }
            changed
        }
    };

    if !has_changes {
        return Ok(());
    }

    let jiten = JitenClient::new(&config);
    jiten.scrape().await?;
    
    let merge = Merge::new(&config, anilist, jiten);
    merge.merge().await?;

    let meili = MeiliClient::new(&config, merge);
    meili.setup().await?;

    Ok(())
}