use std::sync::Arc;
use std::time::Duration;

use reqwest::header::HeaderValue;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::types::{BreakerMeta, BreakersMetadataResponse, RouterMeta, RoutersMetadataResponse};

const DEFAULT_SYNC_INTERVAL: Duration = Duration::from_secs(30);
const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) struct MetadataCache {
    pub breakers: Arc<RwLock<Option<Vec<BreakerMeta>>>>,
    pub routers: Arc<RwLock<Option<Vec<RouterMeta>>>>,
}

pub(crate) struct MetadataHandle {
    pub cache: MetadataCache,
    pub task_handle: tokio::task::JoinHandle<()>,
}

pub(crate) fn start_metadata_sync(
    base_url: String,
    project_id: String,
    api_key: String,
    interval: Option<Duration>,
    cancel: CancellationToken,
) -> MetadataHandle {
    let breakers: Arc<RwLock<Option<Vec<BreakerMeta>>>> = Arc::new(RwLock::new(None));
    let routers: Arc<RwLock<Option<Vec<RouterMeta>>>> = Arc::new(RwLock::new(None));

    let cache = MetadataCache {
        breakers: breakers.clone(),
        routers: routers.clone(),
    };

    let sync_interval = interval.unwrap_or(DEFAULT_SYNC_INTERVAL);

    let task_handle = tokio::spawn(metadata_loop(
        base_url,
        project_id,
        api_key,
        sync_interval,
        cancel,
        breakers,
        routers,
    ));

    MetadataHandle { cache, task_handle }
}

async fn metadata_loop(
    base_url: String,
    project_id: String,
    api_key: String,
    interval: Duration,
    cancel: CancellationToken,
    breakers: Arc<RwLock<Option<Vec<BreakerMeta>>>>,
    routers: Arc<RwLock<Option<Vec<RouterMeta>>>>,
) {
    let http = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .build()
        .unwrap_or_default();

    let breakers_url = format!("{base_url}/v1/projects/{project_id}/breakers/metadata");
    let routers_url = format!("{base_url}/v1/projects/{project_id}/routers/metadata");

    let mut breakers_etag: Option<String> = None;
    let mut routers_etag: Option<String> = None;

    // Initial fetch immediately
    fetch_breakers(&http, &breakers_url, &api_key, &breakers, &mut breakers_etag).await;
    fetch_routers(&http, &routers_url, &api_key, &routers, &mut routers_etag).await;

    let mut tick = tokio::time::interval(interval);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tick.tick().await; // skip first immediate tick

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tick.tick() => {
                let b_stop = fetch_breakers(
                    &http, &breakers_url, &api_key, &breakers, &mut breakers_etag,
                ).await;
                let r_stop = fetch_routers(
                    &http, &routers_url, &api_key, &routers, &mut routers_etag,
                ).await;
                if b_stop || r_stop {
                    warn!("metadata sync stopping due to auth error");
                    break;
                }
            }
        }
    }
}

/// Returns true if syncing should stop (auth error).
async fn fetch_breakers(
    http: &reqwest::Client,
    url: &str,
    api_key: &str,
    cache: &Arc<RwLock<Option<Vec<BreakerMeta>>>>,
    etag: &mut Option<String>,
) -> bool {
    let mut req = http
        .get(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header(
            "X-Contract-Version",
            HeaderValue::from_static(crate::CONTRACT_VERSION),
        );

    if let Some(ref tag) = etag {
        req = req.header("If-None-Match", tag.as_str());
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            match status {
                304 => {
                    debug!("metadata breakers: not modified");
                    false
                }
                200 => {
                    let new_etag = resp
                        .headers()
                        .get("etag")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string());
                    *etag = new_etag;

                    match resp.json::<BreakersMetadataResponse>().await {
                        Ok(parsed) => {
                            *cache.write().await = Some(parsed.breakers);
                            debug!("metadata breakers: updated");
                        }
                        Err(e) => warn!("metadata breakers: parse error: {e}"),
                    }
                    false
                }
                401 | 403 => {
                    warn!("metadata breakers: auth error (HTTP {status})");
                    true
                }
                _ => {
                    warn!("metadata breakers: unexpected status {status}");
                    false
                }
            }
        }
        Err(e) => {
            warn!("metadata breakers: transport error: {e}");
            false
        }
    }
}

/// Returns true if syncing should stop (auth error).
async fn fetch_routers(
    http: &reqwest::Client,
    url: &str,
    api_key: &str,
    cache: &Arc<RwLock<Option<Vec<RouterMeta>>>>,
    etag: &mut Option<String>,
) -> bool {
    let mut req = http
        .get(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header(
            "X-Contract-Version",
            HeaderValue::from_static(crate::CONTRACT_VERSION),
        );

    if let Some(ref tag) = etag {
        req = req.header("If-None-Match", tag.as_str());
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            match status {
                304 => {
                    debug!("metadata routers: not modified");
                    false
                }
                200 => {
                    let new_etag = resp
                        .headers()
                        .get("etag")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string());
                    *etag = new_etag;

                    match resp.json::<RoutersMetadataResponse>().await {
                        Ok(parsed) => {
                            *cache.write().await = Some(parsed.routers);
                            debug!("metadata routers: updated");
                        }
                        Err(e) => warn!("metadata routers: parse error: {e}"),
                    }
                    false
                }
                401 | 403 => {
                    warn!("metadata routers: auth error (HTTP {status})");
                    true
                }
                _ => {
                    warn!("metadata routers: unexpected status {status}");
                    false
                }
            }
        }
        Err(e) => {
            warn!("metadata routers: transport error: {e}");
            false
        }
    }
}
