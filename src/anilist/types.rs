#![allow(dead_code)]
use serde::{Deserialize};

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
pub struct Edge<T> {
    pub edges: T
}

#[derive(Debug, Deserialize)]
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
    pub start_date: Option<FuzzyDate>,
    pub end_date: Option<FuzzyDate>,

    pub next_airing_episode: Option<AiringSchedule>,

    pub studios: Option<Edge<Vec<Studio>>>,
}

#[derive(Debug, Deserialize)]
pub struct StudioNode {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Studio {
    #[serde(rename = "isMain")]
    pub is_main: bool,
    pub node: Option<StudioNode>,
}

#[derive(Debug, Deserialize)]
pub struct Title {
    pub english: Option<String>,
    pub romaji: Option<String>,
    pub native: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AiringSchedule {
    pub episode: i32,
    pub airing_at: i32,
    pub time_until_airing: i32,
}

#[derive(Debug, Deserialize)]
pub struct FuzzyDate {
    pub day: Option<i32>,
    pub month: Option<i32>,
    pub year: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct MediaCoverImage {
    pub medium: Option<String>,
    pub large: Option<String>,
    pub extra_large: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MediaSeason {
    Winter,
    Spring,
    Summer,
    Fall,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MediaStatus {
    Finished,
    Releasing,
    NotYetReleased,
    Cancelled,
    Hiatus,
}