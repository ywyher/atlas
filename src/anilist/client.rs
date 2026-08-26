// src/anilist/client.rs
use super::types::{AnimeQuery, ApiResponse, Media};
use crate::config::Config;
use anyhow::{Context, Result};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use nonzero_ext::*;
use reqwest::header::HeaderMap;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

const QUERY: &str = include_str!("../../graphql/anilist.graphql");
const ANILIST_RATE_LIMIT: u32 = 30;
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
        for attempt in 0..MAX_RETRIES {
            if self.limiter.check().is_err() {
                println!("governor rate limited, waiting...");
                self.limiter.until_ready().await;
            }

            let body = json!({
                "query": QUERY,
                "variables": { "id": id }
            });

            let res = self
                .http
                .post(&self.api_url)
                .json(&body)
                .send()
                .await
                .context("failed to send request to AniList")?;

            if res.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let wait = Self::retry_wait(&res.headers());
                eprintln!(
                    "anilist rate limited (attempt {}/{MAX_RETRIES}), waiting {:?}",
                    attempt + 1,
                    wait
                );
                tokio::time::sleep(wait).await;
                continue;
            }

            if !res.status().is_success() {
                return Err(AniListError::HttpError { status: res.status() }.into());
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