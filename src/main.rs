mod anilist;
mod config;

use anilist::client::{Client, GetMediaResponse};
use anyhow::Result;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let client = Client::new(&config);

    let mut last: Option<GetMediaResponse> = None;
    for i in 1..=100 {
        let res = client.get_media(9253).await?;
        println!("{i}: remaining={:?}", res.headers.get("x-ratelimit-remaining"));
        last = Some(res);
    }

    println!("{:#?}", last);

    Ok(())
}