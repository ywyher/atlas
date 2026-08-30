pub mod consts {
    pub const DATA_FOLDER: &str = "data";
}

mod anilist;
mod config;
mod jiten;
mod merge;

use crate::anilist::client::Client as AnilistClient;
use crate::config::Config;
use crate::jiten::client::Client as JitenClient;
use crate::merge::merge::Merge;
use anyhow::Result;
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("atlas=debug"));

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let config = Config::from_env()?;

    let anilist = AnilistClient::new(&config);
    // anilist.scrape().await?;

    let jiten = JitenClient::new(&config);
    // jiten.scrape().await?;

    let merge = Merge::new(&config, anilist, jiten);
    merge.merge().await?;

    Ok(())
}