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
    pub next_airing_episode: Option<AiringSchedule>,

    pub studios: Option<Edge<Vec<Studio>>>,
    pub characters: Option<Edge<Vec<Option<Character>>>>,
    pub trailer: Option<Trailer>,
    pub external_links: Option<Vec<Option<ExternalLink>>>,
    pub relations: Option<Edge<Vec<Option<Relation>>>>,
    pub recommendations: Option<Edge<Vec<Option<Recommendation>>>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiringSchedule {
    pub episode: Option<i32>,
    pub airing_at: Option<i32>,
    pub time_until_airing: Option<i32>,
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

// ---------------------------- GENERAL
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeneralName {
    pub first: Option<String>,
    pub last: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeneralImage {
    pub large: Option<String>,
}

// ---------------------------- STUDIO

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StudioNode {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Studio {
    #[serde(rename = "isMain")]
    pub is_main: bool,
    pub node: Option<StudioNode>,
}

// ---------------------------- CHARACTER

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Staff {
    pub name: Option<GeneralName>,
    pub image: Option<GeneralImage>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CharacterNode {
    pub name: Option<GeneralName>,
    pub age: Option<String>,
    pub image: Option<GeneralImage>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum CharacterRole {
    Main,
    Supporting,
    Background,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub node: Option<CharacterNode>,
    pub role: Option<CharacterRole>,
    pub voice_actors: Option<Vec<Option<Staff>>>,
}

// ---------------------------- TRAILER

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Trailer {
    pub thumbnail: Option<String>,
    pub id: Option<String>,
    pub site: Option<String>,
}

// ---------------------------- TRAILER

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum ExternalLinkType {
    Info,
    Streaming,
    Social,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExternalLink {
    pub id: i32,
    pub site: String,
    pub site_id: Option<i32>,
    pub r#type: Option<ExternalLinkType>,
    pub color: Option<String>,
    pub url: Option<String>,
}


// ---------------------------- Relation

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationType {
    Adaptation,
    Prequel,
    Sequel,
    Parent,
    SideStory,
    Character,
    Summary,
    Alternative,
    SpinOff,
    Other,
    Source,
    Compilation,
    Contains,
    SameUniverse,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RelationMedia {
    pub id: i32,
    pub title: Option<Title>,
    pub cover_image: Option<MediaCoverImage>,
    pub format: Option<MediaFormat>,
    pub status: Option<MediaStatus>,
    pub start_date: Option<FuzzyDate>,
    pub end_date: Option<FuzzyDate>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Relation {
    pub relation_type: Option<RelationType>,
    pub node: Option<RelationMedia>
}

// ---------------------------- Recommendation


#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationMedia {
    pub id: i32,
    pub title: Option<Title>,
    pub cover_image: Option<MediaCoverImage>,
    pub format: Option<MediaFormat>,
    pub status: Option<MediaStatus>,
    pub season_year: Option<i32>,
    pub average_score: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationNode {
    media_recommendation: Option<RecommendationMedia>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub node: Option<RecommendationNode>
}