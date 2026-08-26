use super::types::{AnimeQuery, ApiResponse, Media};
use crate::{anilist::types::{BatchAnimeQuery, Page}, config::Config};
use anyhow::{Context, Result, anyhow};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use nonzero_ext::*;
use reqwest::{StatusCode, header::HeaderMap};
use serde_json::json;
use tokio::fs;
use std::time::Duration;
use thiserror::Error;

const GET_MEDIA_QUERY: &str = include_str!("../../graphql/anilist.graphql");
const GET_MEDIA_BATCH_QUERY: &str = include_str!("../../graphql/scrape-anilist.graphql");
const ANILIST_RATE_LIMIT: u32 = 29; // anilist rate limits at 30 - keep it under to avoid hitting it by mistake
const ANILIST_PER_PAGE: u32 = 50; // anilist's max
const MAX_RETRIES: u32 = 5;

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
        let quota = Quota::per_minute(nonzero!(ANILIST_RATE_LIMIT))
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

        println!("{:#?}", res.status());
        println!("{:#?}", res.headers());

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

        return Ok(GetMediaResponse { media, headers });
    }

    pub async fn get_media_batch(&self, page: i32) -> Result<GetMediaBatchResponse> {
        let body = json!({
            "query": GET_MEDIA_BATCH_QUERY,
            "variables": { 
                "page": page,
                "perPage": ANILIST_PER_PAGE
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

        // handle rate limiting BEFORE the generic error bail,
        // and without trying to parse a body that won't match BatchAnimeQuery
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Ok(GetMediaBatchResponse {
                page: None,
                headers,
                status_code: status,
            });
        }

        if !status.is_success() {
            anyhow::bail!("AniList returned HTTP {}", status);
        }

        let data: ApiResponse<BatchAnimeQuery> = res
            .json()
            .await
            .context("failed to parse AniList response as JSON")?;
        
        let page: Page = data
            .data
            .and_then(|d| d.page)
            .ok_or_else(|| anyhow!("No media found for this page"))?;

        Ok(GetMediaBatchResponse { page: Some(page), headers, status_code: status })
    }


    pub async fn scrape(&self) -> Result<()> {
        let mut page_number = 1;
        let mut data: Vec<Media> = Vec::new();
        
        for attempt in 0..MAX_RETRIES {
            let mut rate_limited = false;

            loop {
                if self.limiter.check().is_err() {
                    println!("governor rate limited, waiting...");
                    self.limiter.until_ready().await;
                }
                let res = self.get_media_batch(page_number).await?;

                if res.status_code == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    let wait = Self::retry_wait(&res.headers);
                    eprintln!(
                        "anilist rate limited (attempt {}/{MAX_RETRIES}), waiting {:?}",
                        attempt + 1,
                        wait
                    );
                    tokio::time::sleep(wait).await;
                    rate_limited = true;
                    break;
                }

                if !res.status_code.is_success() {
                    return Err(AniListError::HttpError { status: res.status_code }.into());
                }

                let page: Page = res.page.expect("page present on success status");
                data.extend(page.media.iter().cloned());
                println!("{:#?}", page.page_info.current_page);
                println!("{:#?}", res.headers["x-ratelimit-remaining"]);
                println!("{:#?}", data.len());

                if !page.page_info.has_next_page {
                    break;
                }
                page_number += 1;
            }

            if rate_limited {
                continue;
            }

            let json = serde_json::to_string_pretty(&data)?;
            fs::write("output.json", json).await?;
            return Ok(());
        }
        Err(AniListError::RateLimitExceeded.into())
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
}