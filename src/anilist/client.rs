use super::types::{ApiResponse, Media};
use crate::{anilist::types::{BatchAnimeQuery, FuzzyDate, MediaSort, MediaStatus, Page}, config::Config};
use anyhow::{Context, Result, anyhow};
use chrono::{Datelike, NaiveDate};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use nonzero_ext::*;
use reqwest::{StatusCode, header::HeaderMap};
use serde_json::json;
use tokio::{fs, time::Instant};
use tracing::{debug, info, warn};
use std::time::Duration;
use std::collections::HashSet;
use std::path::PathBuf;

const GET_MEDIA_BATCH_QUERY: &str = include_str!("./graphql/media-batch.graphql");
const RATE_LIMIT: u32 = 29; // anilist rate limits at 30
const PER_PAGE: i32 = 50; // anilist's max
const MAX_PAGE: i32 = 5000 / PER_PAGE; // 100
const REQUEST_TIMEOUT_SECS: u64 = 30;
const MAX_RETRIES: u32 = 5;

const FILE_MEDIA: &str = "anilist.json";
const FILE_IDS: &str = "anilist-ids.json";
const FILE_LAST_SCRAPE_TIME_STAMP: &str = "anilist-last-scrape-timestamp.txt";

const SORT_START_DATE: &[MediaSort] = &[MediaSort::StartDate];
const SORT_UPDATED_AT_DESC: &[MediaSort] = &[MediaSort::UpdatedAtDesc];
const STATUS_ACTIVE: &[MediaStatus] = &[MediaStatus::Releasing, MediaStatus::NotYetReleased];

#[derive(Debug)]
pub struct GetMediaBatchResponse {
    pub page: Option<Page>,
    pub headers: HeaderMap,
    pub status_code: StatusCode,
    pub body: Option<String>, // raw body when status isn't success, for logging/retry decisions
}

pub struct Client {
    http: reqwest::Client,
    api_url: String,
    data_folder: String,
    limiter: DefaultDirectRateLimiter,
}


impl Client {
    pub fn new(config: &Config) -> Self {
        let quota = Quota::per_minute(nonzero!(RATE_LIMIT))
            .allow_burst(nonzero!(1u32));

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .expect("failed to build reqwest client");

        Self {
            http,
            api_url: config.anilist_api_url.clone(),
            data_folder: config.data_folder.clone(),
            limiter: RateLimiter::direct(quota),
        }
    }

    pub async fn get_media_batch(
        &self,
        page: i32,
        start: Option<i32>,
        end: Option<i32>,
        sort: &[MediaSort],
        status_in: Option<&[MediaStatus]>,
    ) -> Result<GetMediaBatchResponse> {
        let mut variables = serde_json::Map::new();
        variables.insert("page".to_string(), json!(page));
        variables.insert("perPage".to_string(), json!(PER_PAGE));
        variables.insert("sort".to_string(), json!(sort));

        if let Some(start) = start {
            variables.insert("start".to_string(), json!(start));
        }
        if let Some(end) = end {
            variables.insert("end".to_string(), json!(end));
        }
        if let Some(statuses) = status_in {
            variables.insert("status_in".to_string(), json!(statuses));
        }

        let body = json!({
            "query": GET_MEDIA_BATCH_QUERY,
            "variables": variables,
        });

        let res = self
            .http
            .post(&self.api_url)
            .json(&body)
            .send()
            .await
            .context("failed to send request to AniList")?;

        let status = res.status();
        let headers = res.headers().clone();

        if status == StatusCode::TOO_MANY_REQUESTS {
            let text = res.text().await.unwrap_or_default();
            return Ok(GetMediaBatchResponse {
                page: None,
                headers,
                status_code: status,
                body: Some(text)
            });
        }
        
        if !status.is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow!(
                "AniList returned HTTP {} (body={:.500})",
                status,
                text
            ));
        }

        let text = res.text().await.context("failed to read AniList response body")?;
        let data: ApiResponse<BatchAnimeQuery> = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse AniList response as JSON (body={:.500})", text))?;

        let page: Page = data
            .data
            .and_then(|d| d.page)
            .ok_or_else(|| anyhow!("No media found for this page"))?;

        Ok(GetMediaBatchResponse {
            page: Some(page),
            headers,
            status_code: status,
            body: None,
        })
    }

    async fn fetch_page_with_retries(
        &self,
        page_number: i32,
        start: Option<i32>,
        end: Option<i32>,
        batch: &i32,
        sort: &[MediaSort],
        status_in: Option<&[MediaStatus]>
    ) -> Result<Page> {
        let mut retries = 0;

        loop {
            if self.limiter.check().is_err() {
                debug!("local rate limiter engaged, waiting for next token");
                self.limiter.until_ready().await;
            }

            let time = Instant::now();
            debug!(page = page_number, batch, "requesting batch");
            let res = match self.get_media_batch(page_number, start, end, sort, status_in).await {
                Ok(res) => res,
                Err(e) => {
                    retries += 1;
                    if retries > MAX_RETRIES {
                        return Err(e).context(format!(
                            "exceeded max retries ({MAX_RETRIES}) for page {page_number}"
                        ));
                    }
                    warn!(error = %e, retries = retries, max_retries = MAX_RETRIES, "request failed, retrying after backoff");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };
            debug!(elapsed = ?time.elapsed(), "batch request completed");

            if res.status_code == StatusCode::TOO_MANY_REQUESTS {
                retries += 1;
                if retries > MAX_RETRIES {
                    return Err(anyhow!(
                        "exceeded max retries ({MAX_RETRIES}) for page {page_number} (rate limited)"
                    ));
                }
                let wait = Self::retry_wait(&res.headers);
                warn!(?wait, retries = retries, max_retries = MAX_RETRIES, "AniList rate limited, backing off");
                tokio::time::sleep(wait).await;
                continue;
            }

            if res.status_code.is_server_error() {
                retries += 1;
                if retries > MAX_RETRIES {
                    return Err(anyhow!(
                        "exceeded max retries ({MAX_RETRIES}) for page {page_number}: HTTP {} (body={:.500})",
                        res.status_code,
                        res.body.as_deref().unwrap_or_default()
                    ));
                }
                warn!(
                    status = %res.status_code,
                    body = %res.body.as_deref().unwrap_or_default(),
                    retries = retries,
                    max_retries = MAX_RETRIES,
                    "AniList server error, retrying after backoff"
                );
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            retries = 0;
            return Ok(res.page.expect("page present on success status"));
        }
    }

    async fn scrape_start_date_null(
        &self,
        seen: &mut HashSet<i32>,
        out: &mut Vec<Media>,
        batch: &mut i32,
    ) -> Result<()> {
        let mut page_number = 1;
        let before = out.len();
        info!("starting scrape null start date entries");

        loop {
            let page = self
                .fetch_page_with_retries(page_number, None, None, batch, SORT_START_DATE, None)
                .await?;

            let mut hit_dated_entry = false;
            for m in page.media {
                let is_null = m.start_date.as_ref().map(|d| d.year.is_none()).unwrap_or(true);
                if !is_null {
                    hit_dated_entry = true;
                    break;
                }
                if seen.insert(m.id) {
                    out.push(m);
                }
            }

            if hit_dated_entry || !page.page_info.has_next_page {
                break;
            }

            *batch += 1;
            page_number += 1;
        }

        info!(added = out.len() - before, "scrape null start date entries completed");
        Ok(())
    }

    async fn scrape_range(
        &self,
        start: i32,
        end: i32,
        seen: &mut HashSet<i32>,
        out: &mut Vec<Media>,
        batch: &mut i32,
    ) -> Result<()> {
        let mut buffer: Vec<Media> = Vec::new();
        let mut page_number = 1;

        info!(start, end, "starting scrape range");
        loop {
            let page = self
                .fetch_page_with_retries(page_number, Some(start), Some(end), batch, SORT_START_DATE, None)
                .await?;
            let has_next = page.page_info.has_next_page;
            buffer.extend(page.media);

            if !has_next {
                let before = out.len();
                for m in std::mem::take(&mut buffer) {
                    if seen.insert(m.id) {
                        out.push(m);
                    }
                }
                info!(start, end, added = out.len() - before, "scrape range complete");
                return Ok(());
            }

            if page_number >= MAX_PAGE {
                let cutoff = buffer
                    .last()
                    .and_then(|m| m.start_date.clone())
                    .and_then(|d| Self::fuzzy_date_to_int(&d, -1))
                    .ok_or_else(|| anyhow!("could not compute cutoff date for pagination"))?;

                let before = out.len();
                for m in std::mem::take(&mut buffer) {
                    if seen.insert(m.id) {
                        out.push(m);
                    }
                }
                *batch += 1;

                info!(
                    cutoff, end, batch = *batch, added = out.len() - before,
                    "page limit reached, recursing with new cutoff date"
                );
                return Box::pin(self.scrape_range(cutoff, end, seen, out, batch)).await;
            }

            *batch += 1;
            page_number += 1;
        }
    }

    pub async fn scrape(&self) -> Result<()> {
        let run_started_at = chrono::Utc::now().timestamp();
        let mut seen: HashSet<i32> = HashSet::new();
        let mut data: Vec<Media> = Vec::new();
        let mut batch: i32 = 1;

        let start: i32 = 1940 * 10_000;
        let now = chrono::Local::now().date_naive();
        let future_cutoff = now + chrono::Duration::days(365 * 3);
        let end: i32 = future_cutoff.year() * 10_000 + future_cutoff.month() as i32 * 100 + future_cutoff.day() as i32;
        let time = Instant::now();

        info!(start, end, "starting AniList scrape");
        // query first so null entries are pushed before dated entries
        self.scrape_start_date_null(&mut seen, &mut data, &mut batch).await?;
        self.scrape_range(start, end, &mut seen, &mut data, &mut batch).await?;
        info!(elapsed = ?time.elapsed(), total = data.len(), "AniList scrape complete");

        let json = serde_json::to_string_pretty(&data)?;
        let ids = serde_json::to_string_pretty(&seen)?;

        let dir = PathBuf::from(&self.data_folder);
        let media_path = dir.join(FILE_MEDIA);
        let ids_path = dir.join(FILE_IDS);

        fs::create_dir_all(&dir).await?;
        fs::write(&media_path, json).await?;
        fs::write(&ids_path, ids).await?;

        info!(path = %media_path.display(), "wrote AniList media to disk");
        info!(path = %ids_path.display(), count = seen.len(), "wrote AniList ids to disk");


        let last_scrape_path = dir.join(FILE_LAST_SCRAPE_TIME_STAMP);

        // buffer to absorb clock drift / in-flight edits during this run
        let next_cutoff = run_started_at - 1 * 3600; // subtracts 1 hours
        fs::write(&last_scrape_path, next_cutoff.to_string()).await?;
        info!(next_cutoff, "updated last scrape timestamp");

        Ok(())
    }

    pub async fn scrape_incremental(&self) -> Result<()> {
        let dir = PathBuf::from(&self.data_folder);
        let last_scrape_path = dir.join(FILE_LAST_SCRAPE_TIME_STAMP);
        
        if !last_scrape_path.exists() {
            info!("no previous scrape timestamp found, falling back to full scrape");
            return self.scrape().await;
        }
        
        let run_started_at = chrono::Utc::now().timestamp();
        let time = Instant::now();

        let cutoff_ts: i64 = fs::read_to_string(&last_scrape_path)
            .await
            .context("failed to read last scrape timestamp")?
            .trim()
            .parse()
            .context("failed to parse last scrape timestamp")?;

        let cutoff_dt = chrono::DateTime::from_timestamp(cutoff_ts, 0)
            .context("cutoff_ts is not a valid unix timestamp")?;

        info!(
            cutoff_ts,
            cutoff = %cutoff_dt,
            "starting incremental AniList scrape"
        );
        
        let existing = self.load_media().await.unwrap_or_default();
        let mut seen: HashSet<i32> = existing.iter().map(|m| m.id).collect();
        let mut by_id: std::collections::HashMap<i32, Media> =
            existing.into_iter().map(|m: Media| (m.id, m)).collect();

        let mut batch: i32 = 1; 
        let mut page_number = 1;
        let mut real_changes = 0;
        let mut noise = 0;
        let mut new_entries = 0;

        loop {
            // limiting to NOT_YET_RELEASED, RELEASING means that it wont catch entries shifting from RELEASING to FINISHED
            // although it will be catched through the weekly fully re-scraping, at the cost of that time delay
            let page = self
                .fetch_page_with_retries(page_number, None, None, &batch, SORT_UPDATED_AT_DESC, Some(STATUS_ACTIVE))
                .await?;

            let mut hit_cutoff = false;

            for m in page.media {
                let Some(ts) = m.updated_at else {
                    warn!(id = m.id, "entry missing updated_at, skipping");
                    continue;
                };

                if ts < cutoff_ts {
                    hit_cutoff = true;
                    break;
                }

                match by_id.get(&m.id) {
                    Some(existing) if existing.content_eq(&m) => {
                        noise += 1;
                    }
                    Some(_) => {
                        debug!(id = m.id, "entry content changed");
                        real_changes += 1;
                        by_id.insert(m.id, m);
                    }
                    None => {
                        debug!(id = m.id, "new entry found");
                        new_entries += 1;
                        seen.insert(m.id);
                        by_id.insert(m.id, m);
                    }
                }
            }

            if hit_cutoff || !page.page_info.has_next_page {
                debug!(
                    page = page_number,
                    hit_cutoff,
                    has_next_page = page.page_info.has_next_page,
                    "stopping pagination"
                );
                break;
            }

            batch += 1;
            page_number += 1;
        }

        info!(
            real_changes,
            new_entries,
            noise,
            pages = page_number,
            elapsed = ?time.elapsed(),
            "incremental scrape fetch complete"
        );

        let data: Vec<Media> = by_id.into_values().collect();
        let json = serde_json::to_string_pretty(&data)?;
        let ids = serde_json::to_string_pretty(&seen)?;

        let media_path = dir.join(FILE_MEDIA);
        let ids_path = dir.join(FILE_IDS);

        fs::create_dir_all(&dir).await?;
        fs::write(&media_path, json).await?;
        fs::write(&ids_path, ids).await?;
        info!(path = %media_path.display(), total = data.len(), "wrote AniList media to disk");
        info!(path = %ids_path.display(), count = seen.len(), "wrote AniList ids to disk");

        // buffer to absorb clock drift / in-flight edits during this run
        let next_cutoff = run_started_at - 1 * 3600; // subtracts 1 hours
        fs::write(&last_scrape_path, next_cutoff.to_string()).await?;
        info!(next_cutoff, "updated last scrape timestamp");

        Ok(())
    }

    pub async fn load_media(&self) -> Result<Vec<Media>> {
        let path = PathBuf::from(&self.data_folder).join(FILE_MEDIA);
        debug!(path = %path.display(), "loading anilist media from disk");

        let json = fs::read_to_string(&path).await?;
        let media: Vec<Media> = serde_json::from_str(&json)?;

        debug!(count = media.len(), "loaded anilist media");
        Ok(media)
    }

    /// Figures out how long to wait before retrying after a 429.
    /// Prefers `retry-after` (seconds), falls back to `x-ratelimit-reset`
    /// (unix timestamp), falls back to a flat 60s if neither is present/parseable.
    fn retry_wait(headers: &HeaderMap) -> Duration {
        if let Some(secs) = headers
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            return Duration::from_secs(secs.max(1));
        }

        if let Some(reset_ts) = headers
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let secs = reset_ts.saturating_sub(now).max(1);
            return Duration::from_secs(secs);
        }

        Duration::from_secs(60)
    }

    fn fuzzy_date_to_int(date: &FuzzyDate, margin_days: i64) -> Option<i32> {
        let year = date.year?;

        if margin_days == 0 {
            let month = date.month.unwrap_or(0);
            let day = date.day.unwrap_or(0);
            return Some(year * 10_000 + month * 100 + day);
        }

        let month = date.month.unwrap_or(1);
        let day = date.day.unwrap_or(1);

        let base = NaiveDate::from_ymd_opt(year, month as u32, day as u32)?;
        let shifted = base + chrono::Duration::days(margin_days);

        Some(shifted.year() * 10_000 + shifted.month() as i32 * 100 + shifted.day() as i32)
    }
}