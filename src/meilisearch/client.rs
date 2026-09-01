use anyhow::Result;
use meilisearch_sdk::{client::Client as Meili, errors::ErrorCode, settings::Settings, tasks::Task};
use tokio::time::Instant;
use tracing::{debug, info, warn};
use crate::{config::Config, merge::Merge};

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

    let task = self.client
        .create_index(INDEX_ANIMES, Some("id"))
        .await?
        .wait_for_completion(&self.client, None, None)
        .await?;

    match task {
        Task::Succeeded { .. } => info!(index = INDEX_ANIMES, "index created"),
        Task::Failed { content } if content.error.error_code == ErrorCode::IndexAlreadyExists => {
            debug!(index = INDEX_ANIMES, "index already exists, skipping creation");
        }
        Task::Failed { content } => {
            anyhow::bail!("failed to create index {INDEX_ANIMES}: {}", content.error);
        }
        _ => {}
    }

    let index = self.client.index(INDEX_ANIMES);
    let desired = Self::desired_settings();

    let current = index.get_settings().await?;

    if Self::settings_match(&current, &desired) {
      debug!(index = INDEX_ANIMES, "settings already up to date, skipping");
      return Ok(());
    }

    debug!(index = INDEX_ANIMES, "settings changed, applying...");
    index
        .set_settings(&desired)
        .await?
        .wait_for_completion(&self.client, None, None)
        .await?;
    info!(index = INDEX_ANIMES, "index settings applied");

    Ok(())
  }

  fn desired_settings() -> Settings {
      Settings::new()
          .with_searchable_attributes([
              "id", "title.english", "title.romaji", "title.native", "synonyms",
          ])
          .with_filterable_attributes([
              "genres", "status", "season", "seasonYear", "format", "popularity",
          ])
          .with_sortable_attributes([
              "language_stats.wordCount",
              "language_stats.unique_kanji_count",
              "language_stats.difficulty",
              "language_stats.speechDuration",
              "language_stats.speechSpeed",
          ])
  }

  fn settings_match(current: &Settings, desired: &Settings) -> bool {
      fn normalized(v: &Option<Vec<String>>) -> Vec<String> {
          let mut v = v.clone().unwrap_or_default();
          v.sort();
          v
      }
      
      fn filterable_match(current: &Settings, desired: &Settings) -> bool {
          let mut c = current.filterable_attributes.clone().unwrap_or_default();
          let mut d = desired.filterable_attributes.clone().unwrap_or_default();
          // Only works if FilterableAttribute: Ord
          c.sort_by_key(|a| format!("{:?}", a)); // fallback sort key if no Ord
          d.sort_by_key(|a| format!("{:?}", a));
          c == d // requires PartialEq
      }

      let is_match = normalized(&current.searchable_attributes) == normalized(&desired.searchable_attributes)
          && normalized(&current.sortable_attributes) == normalized(&desired.sortable_attributes)
          && filterable_match(current, desired);

      debug!(is_match, "settings comparison result");
      is_match
  }


  pub async fn add_documents(&self) -> Result<()> {
    let media = &self.merge.load_media().await?;
    let time = Instant::now();
    
    info!(index = INDEX_ANIMES, "adding documents to the index...");
    self.client
      .index(INDEX_ANIMES)
      .add_documents(media, Some("id"))
      .await?;
    info!(index = INDEX_ANIMES, count = media.len(), elapsed = ?time.elapsed(), "add documents to the index");

    Ok(())
  }

  #[allow(dead_code)]
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