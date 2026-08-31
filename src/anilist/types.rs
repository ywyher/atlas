#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename = "Media")]
pub struct ApiResponse<T> {
    pub data: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct AnimeQuery {
    #[serde(rename = "Media")]
    pub media: Option<Media>,
}

#[derive(Debug, Deserialize)]
pub struct BatchAnimeQuery {
    #[serde(rename = "Page")]
    pub page: Option<Page>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub page_info: PageInfo,
    pub media: Vec<Media>
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub total: i32,
    pub last_page: i32,
    pub has_next_page: bool,
}


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Edge<T> {
    pub edges: T
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub id: i32,
    pub id_mal: Option<i32>,
    pub title: Option<Title>,
    pub cover_image: Option<MediaCoverImage>,
    pub format: Option<MediaFormat>,
    pub status: Option<MediaStatus>,
    pub season: Option<MediaSeason>,
    pub season_year: Option<i32>,
    pub synonyms: Option<Vec<Option<String>>>,
    pub banner_image: Option<String>,
    pub description: Option<String>,
    pub episodes: Option<i32>,
    pub duration: Option<i32>,
    pub genres: Option<Vec<Option<String>>>,
    pub average_score: Option<i32>,
    pub popularity: Option<i32>,
    
    pub start_date: Option<FuzzyDate>,
    pub end_date: Option<FuzzyDate>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Title {
    pub english: Option<String>,
    pub romaji: Option<String>,
    pub native: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FuzzyDate {
    pub day: Option<i32>,
    pub month: Option<i32>,
    pub year: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MediaCoverImage {
    pub medium: Option<String>,
    pub large: Option<String>,
    pub extra_large: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaFormat {
    Tv,
    TvShort,
    Movie,
    Special,
    Ova,
    Ona,
    Music,
    Manga,
    Novel,
    OneShot,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum MediaSeason {
    Winter,
    Spring,
    Summer,
    Fall,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaStatus {
    Finished,
    Releasing,
    NotYetReleased,
    Cancelled,
    Hiatus,
}