use std::collections::HashMap;
use anyhow::Result;
use tokio::fs;
use crate::anilist::types::{Media as AnilistMedia};
use crate::jiten::types::{Media as JitenMedia};

pub struct Merge {

}

impl Merge {
  fn merge_one(
    m: AnilistMedia,
    jiten_by_deck_id: &HashMap<i64, JitenMedia>,
    anilist_to_deck: &HashMap<i64, i64>,
  ) -> Result<()> {
    // let language_stats = anilist_to_deck
    //     .get(&(m.id as i64))
    //     .and_then(|deck_id| jiten_by_deck_id.get(deck_id))
    //     .map(LanguageStats::from);

    Ok(())
  }

  pub async fn load_data() -> Result<()> {
    let anilist_json = fs::read_to_string("data/anilist.json").await?;
    let anilist_media: Vec<AnilistMedia> = serde_json::from_str(&anilist_json)?;
  
    // let jiten_json = fs::read_to_string("data/jiten.json").await?;
    // let jiten_media: Vec<JitenMedia> = serde_json::from_str(&jiten_json)?;
    // let jiten_media_by_id: HashMap<i64, JitenMedia> = jiten_media
    //   .into_iter()
    //   .map(|m| (m.deck_id, m))
    //   .collect();

    // println!("{:#?}", anilist_media);
    // println!("{:#?}", jiten_media_by_id);

    Ok(())
  }
}