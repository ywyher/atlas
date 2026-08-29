mod anilist;
mod config;
mod jiten;

use anilist::client::{Client as AnilistClient};
use jiten::client::{Client as JitenClient};
use anyhow::Result;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    
    let anilist = AnilistClient::new(&config);
    anilist.scrape().await?;
    
    let jiten = JitenClient::new(&config);
    jiten.scrape().await?;
    
    Ok(())
}