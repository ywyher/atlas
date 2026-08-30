use anyhow::{Result, Context};
use tokio::{fs, time::Instant};
use tracing::{debug, info};
use crate::{jiten::types::{Media, JITEN_LINK_VALUES}, config::Config};
use std::collections::HashMap;
use crate::consts::DATA_FOLDER;
use std::path::PathBuf;

pub struct Client {
    api_url: String,
    data_folder: String,
}

const FILE_MEDIA: &str = "jiten.json";
const FILE_IDS: &str = "anilist-to-jiten.json";

impl Client {
    pub fn new(config: &Config) -> Self {
        Self {
            api_url: config.jiten_api_url.clone(),
            data_folder: config.data_folder.clone()
        }
    }

    pub async fn scrape(&self) -> Result<()> {
        let api_url = &self.api_url;
        let endpoint = format!("{api_url}/api/media-deck/get-media-decks-by-type/anime");

        info!(endpoint = %endpoint, "starting jiten scrape");

        let time = Instant::now();
        let res = reqwest::get(endpoint).await?;
        let data: Vec<Media> = res.json().await?;

        info!(elapsed = ?time.elapsed(), count = data.len(), "jiten scrape complete");

        let json = serde_json::to_string_pretty(&data)?;
        let ids_map = self.map_to_anilist_ids(Some(&data)).await?;
        let ids_json = serde_json::to_string_pretty(&ids_map)?;

        let dir = PathBuf::from(&self.data_folder);
        let media_path = dir.join(FILE_MEDIA);
        let ids_path = dir.join(FILE_IDS);

        fs::create_dir_all(&dir).await?;
        fs::write(&media_path, json).await?;
        fs::write(&ids_path, ids_json).await?;

        info!(path = %media_path.display(), "wrote jiten media to disk");
        info!(path = %ids_path.display(), count = ids_map.len(), "wrote anilist-to-jiten id map to disk");

        Ok(())
    }

    /// Builds a map of anilist_id -> deck_id.
    /// If `media` is provided, uses it directly (no file read).
    /// Otherwise falls back to reading FILE_MEDIA from disk.
    pub async fn map_to_anilist_ids(&self, media: Option<&[Media]>) -> Result<HashMap<i64, i64>> {
        let owned;
        let media: &[Media] = match media {
            Some(m) => m,
            None => {
                debug!("media not provided, reading from disk");
                let path = format!("{DATA_FOLDER}/{FILE_MEDIA}");
                let contents = fs::read_to_string(path)
                    .await
                    .context("Should have been able to read the file")?;

                owned = serde_json::from_str::<Vec<Media>>(&contents)
                    .context("Failed to parse JSON")?;
                &owned
            }
        };

        Ok(self.build_anilist_map(media))
    }

    fn build_anilist_map(&self, media: &[Media]) -> HashMap<i64, i64> {
        let anilist_value = *JITEN_LINK_VALUES.get("Anilist").unwrap();

        media
            .iter()
            .filter_map(|m| {
                let anilist = m.links.iter().find(|x| x.link_type == anilist_value)?;
                let anilist_id = self.extract_anilist_id(&anilist.url).ok()?;
                Some((anilist_id, m.deck_id))
            })
            .collect()
    }

    fn extract_anilist_id(&self, url: &str) -> Result<i64> {
        url.trim_end_matches('/')
            .rsplit('/')
            .next()
            .context("URL has no path segments")?
            .parse::<i64>()
            .context("Failed to parse anilist id as i64")
    }
}