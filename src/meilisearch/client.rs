use anyhow::Result;
use meilisearch_sdk::{client::Client as Meili, settings::Settings};
use tokio::time::Instant;
use tracing::{debug, info, warn};

use crate::{config::Config, merge::{merge::Merge, types::MergedMedia}};

const INDEX_ANIMES: &str = "animes";

pub struct Client {
  client: Meili,
  merge: Merge
}

impl Client {
  pub fn new(config: &Config, merge: Merge) -> Self {
    Self {
      client: Meili::new(&config.meili_url, Some(&config.meili_master_key)).unwrap(),
      merge,
    }
  }

  pub async fn setup(&self) -> Result<()> {
    self.setup_index().await?;
    self.add_documents().await?;

    Ok(())
  }

  pub async fn setup_index(&self) -> Result<()> {
    debug!(index = INDEX_ANIMES, "creating index");

    self.client
      .create_index(INDEX_ANIMES, Some("id"))
      .await?
      .wait_for_completion(&self.client, None, None)
      .await?;

    info!(index = INDEX_ANIMES, "index created");

    let index = self.client.index(INDEX_ANIMES);
    let settings = Settings::new()
      .with_searchable_attributes([
        "id",
        "title.english",
        "title.romaji",
        "title.native",
      ])
      .with_filterable_attributes([
        "genres",
        "status",
        "season",
        "seasonYear",
        "format",
        "popularity",
      ])
      .with_sortable_attributes([
        "language_stats.wordCount",
        "language_stats.unique_kanji_count",
        "language_stats.difficulty",
        "language_stats.speechDuration",
        "language_stats.speechSpeed",
      ]);

    debug!(index = INDEX_ANIMES, "applying index settings");

    index
      .set_settings(&settings)
      .await?
      .wait_for_completion(&self.client, None, None)
      .await?;

    info!(index = INDEX_ANIMES, "index settings applied");

    Ok(())
  }

  pub async fn add_documents(&self) -> Result<()> {
    let media = &self.merge.load_media().await?;
    let time = Instant::now();
    
    info!(index = INDEX_ANIMES, "adding documents to the index...");
    let documents = &self.client
      .index(INDEX_ANIMES)
      .add_documents(&media, Some("id"))
      .await
      .unwrap();
    info!(index = INDEX_ANIMES, count = media.len(), elapsed = ?time.elapsed(), "add documents to the index");

    Ok(())
  }

  pub async fn nuke(&self) -> Result<()> {
    warn!(index = INDEX_ANIMES, "deleting index - all documents and settings will be lost");

    self.client
      .delete_index(INDEX_ANIMES)
      .await?
      .wait_for_completion(&self.client, None, None)
      .await?;

    info!(index = INDEX_ANIMES, "index deleted");

    Ok(())
  }
}