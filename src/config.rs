use anyhow::{Context, Result};
use std::env;

pub struct Config {
  pub anilist_api_url: String,
}

impl Config {
  pub fn from_env() -> Result<Self> {
    dotenvy::dotenv().ok();
    Ok(Self {
        anilist_api_url: env::var("ANILIST_API_URL")
            .context("ANILIST_API_URL must be set in .env")?,
    })
  }
}