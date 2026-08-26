mod anilist;
mod config;

use anilist::client::{Client, GetMediaResponse};
use anyhow::Result;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let client = Client::new(&config);
    client.scrape().await?;
    
    Ok(())
}