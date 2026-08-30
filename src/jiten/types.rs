use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkObject {
  pub link_id: i64,
  pub link_type: i32, // after being mapped
  pub url: String,
  pub deck_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagObject {
  pub tag_id: i64,
  pub name: String, // too lazy to list actual literal values
  pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageStats {
  pub deck_id: i64,
  pub word_count: i64,
  pub unique_word_count: i64,
  pub unique_word_used_once_count: i64,
  pub unique_kanji_count: i64,
  pub unique_kanji_used_once_count: i64,
  pub difficulty: f64,
  pub difficulty_raw: f64,
  pub difficulty_algorithmic: f64,
  pub speech_duration: f64,
  pub speech_mora_count: i64,
  pub speech_speed: f64,
  pub distinct_voter_count: i64, //community_vote_counts
  pub user_adjustment: f64, // difficultyCommunityAdjustment
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
  // deck_id: i64, // included in jiten_language_stats

  pub creation_date: String,
  pub release_date: String,
  pub cover_name: String,

  // after being mapped
  pub media_type: i64,
  pub genres: Vec<i64>,

  pub original_title: String,
  pub romaji_title: String,
  pub english_title: Option<String>,
  pub description: String,

  pub deck_id: i64,
  pub word_count: i64,
  pub unique_word_count: i64,
  pub unique_word_used_once_count: i64,
  pub unique_kanji_count: i64,
  pub unique_kanji_used_once_count: i64,
  pub difficulty: f64,
  pub difficulty_raw: f64,
  pub difficulty_algorithmic: f64,
  pub speech_duration: f64,
  pub speech_mora_count: i64,
  pub speech_speed: f64,
  pub distinct_voter_count: i64,
  pub user_adjustment: f64,

  pub sentence_count: i64,
  pub difficulty_override: f64,
  pub average_sentence_length: f64,

  pub parent_deck_id: Option<i64>,

  pub links: Vec<LinkObject>,
  pub aliases: Vec<String>,
  pub dictionary_entries: Vec<Value>, // empty in sample - shape unconfirmed
  pub relationships: Vec<Value>,

  pub children_deck_count: i64,
  pub selected_word_occurrences: i64,
  pub dialogue_percentage: f64,
  pub hide_dialogue_percentage: bool,
  pub hide_average_sentence_length: bool,

  pub coverage: f64,
  pub unique_coverage: f64,
  pub young_coverage: f64,
  pub young_unique_coverage: f64,
  pub external_rating: f64,

  pub example_sentence: Option<String>,

  pub tags: Vec<TagObject>,

  pub status: Option<i64>, // user-specific - likely maps to jiten_status once logged in
  pub is_favourite: Option<bool>,
  pub is_ignored: Option<bool>,

  pub original_file_name: Option<String>,
}


pub static JITEN_LINK_VALUES: LazyLock<HashMap<&'static str, i32>> =
    LazyLock::new(|| {
        HashMap::from([
            ("Anilist", 4),
            ("Web", 1),
            ("Vndb", 2),
            ("Tmdb", 3),
            ("Mal", 5),
            ("GoogleBooks", 6),
            ("Imdb", 7),
            ("Igdb", 8),
            ("Syosetsu", 9),
            ("Bookmeter", 10),
            ("Amazon", 11),
        ])
    });

impl From<&Media> for LanguageStats {
    fn from(m: &Media) -> Self {
        LanguageStats {
          deck_id: m.deck_id,
          word_count: m.word_count,
          unique_word_count: m.unique_word_count,
          unique_word_used_once_count: m.unique_word_used_once_count,
          unique_kanji_count: m.unique_kanji_count,
          unique_kanji_used_once_count: m.unique_kanji_used_once_count,
          difficulty: m.difficulty,
          difficulty_raw: m.difficulty_raw,
          difficulty_algorithmic: m.difficulty_algorithmic,
          speech_duration: m.speech_duration,
          speech_mora_count: m.speech_mora_count,
          speech_speed: m.speech_speed,
          distinct_voter_count: m.distinct_voter_count,
          user_adjustment: m.user_adjustment,
        }
    }
}