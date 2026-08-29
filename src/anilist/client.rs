use super::types::{AnimeQuery, ApiResponse, Media};
use crate::{anilist::types::{BatchAnimeQuery, FuzzyDate, Page}, config::Config};
use anyhow::{Context, Result, anyhow};
use chrono::{Datelike, Local, NaiveDate};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use nonzero_ext::*;
use reqwest::{StatusCode, header::HeaderMap};
use serde_json::json;
use tokio::{fs, time::Instant};
use std::time::Duration;
use std::collections::HashSet;
use thiserror::Error;

const GET_MEDIA_QUERY: &str = include_str!("./graphql/media.graphql");
const GET_MEDIA_BATCH_QUERY: &str = include_str!("./graphql/media-batch.graphql");
const RATE_LIMIT: u32 = 29; // anilist rate limits at 30
const PER_PAGE: i32 = 50; // anilist's max
const MAX_PAGE: i32 = 5000 / PER_PAGE; // 100
const FOLDER: &str = "data";
const FILE_MEDIA: &str = "anilist.json";
const FILE_IDS: &str = "anilist-ids.json";

#[derive(Debug, Error)]
pub enum AniListError {
    #[error("AniList returned HTTP {status}")]
    HttpError { status: reqwest::StatusCode },

    #[error("rate limited, exceeded max retries")]
    RateLimitExceeded,

    #[error("no media found for id {0}")]
    NotFound(i32),
}

#[derive(Debug)]
pub struct GetMediaResponse {
    pub media: Media,
    pub headers: HeaderMap,
}

#[derive(Debug)]
pub struct GetMediaBatchResponse {
    pub page: Option<Page>,
    pub headers: HeaderMap,
    pub status_code: StatusCode
}

pub struct Client {
    http: reqwest::Client,
    api_url: String,
    limiter: DefaultDirectRateLimiter,
}

impl Client {
    pub fn new(config: &Config) -> Self {
        let quota = Quota::per_minute(nonzero!(RATE_LIMIT))
            .allow_burst(nonzero!(1u32));
        
        Self {
            http: reqwest::Client::new(),
            api_url: config.anilist_api_url.clone(),
            limiter: RateLimiter::direct(quota),
        }
    }

    pub async fn get_media(&self, id: i32) -> Result<GetMediaResponse> {
        let body = json!({
            "query": GET_MEDIA_QUERY,
            "variables": { "id": id }
        });

        let res = self
            .http
            .post(&self.api_url)
            .json(&body)
            .send()
            .await
            .context("failed to send request to AniList")?;

        if !res.status().is_success() {
            anyhow::bail!("AniList returned HTTP {}", res.status());
        }

        let headers = res.headers().clone();
        let data: ApiResponse<AnimeQuery> = res
            .json()
            .await
            .context("failed to parse AniList response as JSON")?;

        let media = data
            .data
            .and_then(|d| d.media)
            .ok_or(AniListError::NotFound(id))?;

        println!("{:#?}", media);

        return Ok(GetMediaResponse { media, headers });
    }

    pub async fn get_media_batch(
        &self, 
        page: i32,
        start: i32,
        end: i32,
    ) -> Result<GetMediaBatchResponse> {
        let body = json!({
            "query": GET_MEDIA_BATCH_QUERY,
            "variables": {
                "page": page,
                "perPage": PER_PAGE,
                "start": start,
                "end": end,
            }
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
            return Ok(GetMediaBatchResponse {
                page: None,
                headers,
                status_code: status,
            });
        }

        let data: ApiResponse<BatchAnimeQuery> = res
            .json()
            .await
            .context("failed to parse AniList response as JSON")?;

        let page: Page = data
            .data
            .and_then(|d| d.page)
            .ok_or_else(|| anyhow!("No media found for this page"))?;

        Ok(GetMediaBatchResponse {
            page: Some(page),
            headers,
            status_code: status,
        })
    }

    async fn scrape_range(
        &self,
        start: i32,
        end: i32,
        seen: &mut HashSet<i32>,
        out: &mut Vec<Media>,
        batch: &mut i32
    ) -> Result<()> {
        let mut buffer: Vec<Media> = Vec::new();
        let mut page_number = 1;
        
        println!("-------- scrape {start} - {end}");
        
        loop {
            let time = Instant::now();
            if self.limiter.check().is_err() {
                println!("governor rate limited, waiting...");
                self.limiter.until_ready().await;
            }
            println!("Requesting Batch {batch}");
            let res = self.get_media_batch(page_number, start, end).await?;
            println!("Batch {batch} took {:?}", time.elapsed());

            if res.status_code == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let wait = Self::retry_wait(&res.headers);
                eprintln!(
                    "anilist rate limited), waiting {:#?}", wait
                );
                tokio::time::sleep(wait).await;
                continue;
            }

            if !res.status_code.is_success() {
                return Err(AniListError::HttpError { status: res.status_code }.into());
            }

            let page = res.page.expect("page present on success status");
            let has_next = page.page_info.has_next_page;
            buffer.extend(page.media);

            if !has_next {
                // < 5000 entries, no issues
                // window fit entirely under the cap - keep everything, done
                for m in std::mem::take(&mut buffer) {
                    if seen.insert(m.id) {
                        out.push(m);
                    }
                }
                return Ok(());
            }

            if page_number >= MAX_PAGE {
                let last_start_date = buffer.last().and_then(|m| m.start_date.clone());
                let cutoff = last_start_date
                    .and_then(|d| Self::fuzzy_date_to_int(&d, -1))
                    .ok_or_else(|| anyhow!("could not compute cutoff date for pagination"))?;

                for m in std::mem::take(&mut buffer) {
                    if seen.insert(m.id) {
                        out.push(m);
                    }
                }
                
                *batch += 1;

                println!("##### Recursion");
                return Box::pin(self.scrape_range(cutoff, end, seen, out, batch)).await;
            }

            *batch += 1;
            page_number += 1;
        }
    }

    pub async fn scrape(&self) -> Result<()> {
        let mut seen: HashSet<i32> = HashSet::new();
        let mut data: Vec<Media> = Vec::new();
        let mut batch: i32 = 1;

        let start: i32 = 1940 * 10_000;
        let now = chrono::Local::now().date_naive();
        let tomorrow = now + chrono::Duration::days(1);
        let end: i32 = tomorrow.year() * 10_000 + tomorrow.month() as i32 * 100 + tomorrow.day() as i32;
        let time = Instant::now();

        self.scrape_range(start, end, &mut seen, &mut data, &mut batch).await?;
        println!("Scraping took {:?}", time.elapsed());

        let json = serde_json::to_string_pretty(&2)?;
        let ids = serde_json::to_string_pretty(&seen)?;
        fs::create_dir_all(FOLDER).await?;
        fs::write(format!("{FOLDER}/{FILE_MEDIA}"), json).await?;
        fs::write(format!("{FOLDER}/{FILE_IDS}"), ids).await?;
        Ok(())
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

        // No margin: keep your original behavior, unknown month/day stay 0
        if margin_days == 0 {
            let month = date.month.unwrap_or(0);
            let day = date.day.unwrap_or(0);
            return Some(year * 10_000 + month * 100 + day);
        }

        // Applying a margin needs a real date to shift, so fill in
        // missing month/day with sane defaults (1st of month/year)
        let month = date.month.unwrap_or(1);
        let day = date.day.unwrap_or(1);

        let base = NaiveDate::from_ymd_opt(year, month as u32, day as u32)?;
        let shifted = base + chrono::Duration::days(margin_days);

        Some(shifted.year() * 10_000 + shifted.month() as i32 * 100 + shifted.day() as i32)
    }

    fn today_plus_buffer() -> i32 {
        let now = Local::now();
        // one year out, to catch already-announced future releases
        (now.year() + 1) as i32 * 10000 + (now.month() as i32) * 100 + now.day() as i32
    }
}
