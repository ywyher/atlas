use anyhow::{Context, Result};
use std::env;
use crate::consts::DATA_FOLDER;

pub struct Config {
    pub anilist_api_url: String,
    pub jiten_api_url: String,
    pub jiten_api_key: String,

    pub meili_url: String,
    pub meili_master_key: String,

    pub data_folder: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::from_filename(".env").ok();
        dotenvy::from_filename_override(".env.local").ok();
        Ok(Self {
            anilist_api_url: env::var("ANILIST_API_URL")
                .context("ANILIST_API_URL must be set in .env")?,
            jiten_api_url: env::var("JITEN_API_URL")
                .context("JITEN_API_URL must be set in .env")?,
            jiten_api_key: env::var("JITEN_API_KEY")
                .context("JITEN_API_KEY must be set in .env")?,

            meili_url: env::var("MEILI_URL")
                .context("MEILI_URL must be set in .env")?,

            meili_master_key: env::var("MEILI_MASTER_KEY")
                .context("MEILI_MASTER_KEY must be set in .env")?,

            data_folder: env::var("DATA_FOLDER").unwrap_or_else(|_| DATA_FOLDER.to_string()),
        })
    }
}