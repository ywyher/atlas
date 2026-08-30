use std::collections::HashMap;
use anyhow::Result;
use tokio::{fs, time::Instant};
use tracing::{debug, info, warn};
use crate::anilist::{types::{Media as AnilistMedia}, client::Client as AnilistClient};
use crate::jiten::{types::{LanguageStats, Media as JitenMedia}, client::Client as JitenClient};
use crate::config::Config;
use crate::merge::types::{LoadDataResponse, MergedMedia};
use std::path::PathBuf;

const FILE: &str = "merged.json";

pub struct Merge {
    data_folder: String,
    anilist: AnilistClient,
    jiten: JitenClient,
}

impl Merge {
    pub fn new(config: &Config, anilist: AnilistClient, jiten: JitenClient) -> Self {
        Self {
            data_folder: config.data_folder.clone(),
            anilist,
            jiten,
        }
    }
    
    pub async fn merge(&self) -> Result<()> {
        info!("starting merge");
        let time = Instant::now();

        let data: LoadDataResponse = Self::load_data(&self).await?;

        let merged = data
            .anilist_media
            .iter()
            .map(|m| {
                Self::merge_one(m, &data.jiten_media_by_id, &data.anilist_to_jiten)
            })
            .collect::<Result<Vec<MergedMedia>>>()?;

        let matched = merged.iter().filter(|m| m.language_stats.is_some()).count();
        info!(
            elapsed = ?time.elapsed(),
            total = merged.len(),
            matched,
            unmatched = merged.len() - matched,
            "merge complete"
        );

        let json = serde_json::to_string_pretty(&merged)?;
        
        let dir = PathBuf::from(&self.data_folder);
        let file = dir.join(FILE);

        fs::create_dir_all(&dir).await?;
        fs::write(&file, json).await?;

        info!(path = %file.display(), "wrote merged media to disk");

        Ok(())
    }

    pub fn merge_one(
        anilist_media: &AnilistMedia,
        jiten_media_by_id: &HashMap<i64, JitenMedia>,
        anilist_to_jiten: &HashMap<i64, i64>,
    ) -> Result<MergedMedia> {
        let language_stats = anilist_to_jiten
            .get(&(anilist_media.id as i64))
            .and_then(|jiten_id| jiten_media_by_id.get(jiten_id))
            .map(LanguageStats::from);

        if language_stats.is_none() {
            debug!(anilist_id = anilist_media.id, "no jiten language stats found for anilist id");
        }

        Ok(MergedMedia {
            anime: anilist_media.clone(),
            language_stats,
        })
    }

    async fn load_data(&self) -> Result<LoadDataResponse> {
        let anilist_media = self.anilist.load_media().await?;
        let jiten_media_by_id = self.jiten.load_media().await?;
        let anilist_to_jiten = self.jiten.load_id_map().await?;

        if anilist_media.is_empty() {
            warn!("loaded anilist media list is empty");
        }

        Ok(LoadDataResponse {
            anilist_media,
            jiten_media_by_id,
            anilist_to_jiten,
        })
    }

    pub async fn load_media(&self) -> Result<Vec<MergedMedia>> {
        let path = PathBuf::from(&self.data_folder).join(FILE);
        debug!(path = %path.display(), "loading merged media from disk");

        let json = fs::read_to_string(&path).await?;
        let merged: Vec<MergedMedia> = serde_json::from_str(&json)?;

        debug!(count = merged.len(), "loaded merged merged");
        Ok(merged)
    }
}