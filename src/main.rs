mod anilist;
mod config;
mod jiten;
mod merge;

use anilist::client::{Client as AnilistClient};
use jiten::client::{Client as JitenClient};
use config::Config;
use anyhow::Result;
use tracing_subscriber::{self, EnvFilter};
use merge::merge::{Merge};

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("atlas=debug"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    let config = Config::from_env()?;

    let anilist = AnilistClient::new(&config);
    anilist.scrape().await?;
    
    // let jiten = JitenClient::new(&config);
    // jiten.scrape().await?;

    // Merge::load_data().await?;

    
    Ok(())
}