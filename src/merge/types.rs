use serde::{Serialize, Deserialize};
use crate::{anilist::types::Media as AnilistMedia, jiten::types::{LanguageStats, Media as JitenMedia}};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MergedMedia {
    #[serde(flatten)]
    pub anime: AnilistMedia,
    pub language_stats: Option<LanguageStats>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoadDataResponse {
    pub anilist_media: Vec<AnilistMedia>,
    pub jiten_media_by_id: HashMap<i64, JitenMedia>,
    pub anilist_to_jiten: HashMap<i64, i64>,
}