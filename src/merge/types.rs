use serde::{Serialize, Deserialize};
use crate::{anilist::types::Media as AnilistMedia, jiten::types::LanguageStats};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MergedMedia {
    #[serde(flatten)]
    pub anime: AnilistMedia,
    pub language_stats: Option<LanguageStats>,
}