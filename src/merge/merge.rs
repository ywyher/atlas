use std::collections::HashMap;
use anyhow::Result;
use tokio::{fs, time::Instant};
use tracing::{debug, info, warn};
use crate::anilist::types::{Media as AnilistMedia};
use crate::config::Config;
use crate::jiten::types::{LanguageStats, Media as JitenMedia};
use crate::merge::types::{LoadDataResponse, MergedMedia};
use std::path::PathBuf;

const FILE: &str = "merged.json";

pub struct Merge {
    data_folder: String,
}

impl Merge {
    pub fn new(config: &Config) -> Self {
        Self { data_folder: config.data_folder.clone() }
    }
    
    pub async fn merge(&self) -> Result<()> {
        info!("starting merge");
        let time = Instant::now();

        let data: LoadDataResponse = Self::load_data().await?;

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

    async fn load_data() -> Result<LoadDataResponse> {
        debug!("loading anilist media from disk");
        let anilist_json = fs::read_to_string("data/anilist.json").await?;
        let anilist_media: Vec<AnilistMedia> = serde_json::from_str(&anilist_json)?;
        debug!(count = anilist_media.len(), "loaded anilist media");

        debug!("loading jiten media from disk");
        let jiten_json = fs::read_to_string("data/jiten.json").await?;
        let jiten_media: Vec<JitenMedia> = serde_json::from_str(&jiten_json)?;
        debug!(count = jiten_media.len(), "loaded jiten media");

        let jiten_media_by_id: HashMap<i64, JitenMedia> = jiten_media
            .into_iter()
            .map(|m| (m.deck_id, m))
            .collect();

        debug!("loading anilist-to-jiten id map from disk");
        let anilist_to_jiten_json = fs::read_to_string("data/anilist-to-jiten.json").await?;
        let anilist_to_jiten: HashMap<i64, i64> = serde_json::from_str(&anilist_to_jiten_json)?;
        debug!(count = anilist_to_jiten.len(), "loaded anilist-to-jiten map");

        if anilist_media.is_empty() {
            warn!("loaded anilist media list is empty");
        }

        Ok(LoadDataResponse {
            anilist_media,
            jiten_media_by_id,
            anilist_to_jiten,
        })
    }
}