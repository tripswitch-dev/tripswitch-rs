use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use reqwest_eventsource::{Event, EventSource};
use tokio::sync::{Notify, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::types::{BreakerStateEntry, BreakerStateValue};

pub(crate) struct SseStats {
    pub connected: Arc<AtomicBool>,
    pub reconnects: Arc<AtomicU64>,
}

pub(crate) struct SseHandle {
    pub states: Arc<RwLock<HashMap<String, BreakerStateEntry>>>,
    pub stats: SseStats,
    pub task_handle: tokio::task::JoinHandle<()>,
}

pub(crate) type StateChangeCallback =
    Arc<dyn Fn(&str, BreakerStateValue, BreakerStateValue) + Send + Sync>;

pub(crate) fn start_sse_listener(
    base_url: String,
    project_id: String,
    api_key: String,
    cancel: CancellationToken,
    ready: Arc<Notify>,
    on_state_change: Option<StateChangeCallback>,
) -> SseHandle {
    let states: Arc<RwLock<HashMap<String, BreakerStateEntry>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let connected = Arc::new(AtomicBool::new(false));
    let reconnects = Arc::new(AtomicU64::new(0));

    let stats = SseStats {
        connected: connected.clone(),
        reconnects: reconnects.clone(),
    };

    let states_clone = states.clone();
    let task_handle = tokio::spawn(sse_loop(SseLoopParams {
        base_url,
        project_id,
        api_key,
        cancel,
        ready,
        states: states_clone,
        connected,
        reconnects,
        on_state_change,
    }));

    SseHandle {
        states,
        stats,
        task_handle,
    }
}

struct SseLoopParams {
    base_url: String,
    project_id: String,
    api_key: String,
    cancel: CancellationToken,
    ready: Arc<Notify>,
    states: Arc<RwLock<HashMap<String, BreakerStateEntry>>>,
    connected: Arc<AtomicBool>,
    reconnects: Arc<AtomicU64>,
    on_state_change: Option<StateChangeCallback>,
}

async fn sse_loop(p: SseLoopParams) {
    let SseLoopParams {
        base_url,
        project_id,
        api_key,
        cancel,
        ready,
        states,
        connected,
        reconnects,
        on_state_change,
    } = p;
    let url = format!("{base_url}/v1/projects/{project_id}/breakers/state:stream");
    let mut first_event = true;
    let backoffs: &[Duration] = &[
        Duration::from_secs(1),
        Duration::from_secs(2),
        Duration::from_secs(4),
        Duration::from_secs(8),
        Duration::from_secs(15),
        Duration::from_secs(30),
    ];
    let mut backoff_idx: usize = 0;

    loop {
        if cancel.is_cancelled() {
            break;
        }

        debug!("connecting to SSE stream: {url}");

        let client = reqwest::Client::builder()
            .http1_only()
            .build()
            .unwrap_or_default();

        let req = client
            .get(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header(
                "X-Contract-Version",
                crate::CONTRACT_VERSION,
            );

        let mut es = EventSource::new(req).unwrap();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    connected.store(false, Ordering::Relaxed);
                    es.close();
                    return;
                }
                event = es.next() => {
                    match event {
                        Some(Ok(Event::Open)) => {
                            debug!("SSE connection opened");
                            connected.store(true, Ordering::Relaxed);
                            backoff_idx = 0;
                        }
                        Some(Ok(Event::Message(msg))) => {
                            match serde_json::from_str::<BreakerStateEntry>(&msg.data) {
                                Ok(entry) => {
                                    let name = entry.breaker.clone();
                                    let new_state = entry.state;

                                    let old_state = {
                                        let mut map = states.write().await;
                                        let old = map.get(&name).map(|e| e.state);
                                        map.insert(name.clone(), entry);
                                        old
                                    };

                                    // Fire state change callback on transitions
                                    if let Some(ref cb) = on_state_change {
                                        if let Some(old) = old_state {
                                            if old != new_state {
                                                cb(&name, old, new_state);
                                            }
                                        }
                                    }

                                    if first_event {
                                        first_event = false;
                                        ready.notify_waiters();
                                    }
                                }
                                Err(e) => {
                                    warn!("failed to parse SSE event data: {e}");
                                }
                            }
                        }
                        Some(Err(e)) => {
                            warn!("SSE error: {e}");
                            connected.store(false, Ordering::Relaxed);
                            break;
                        }
                        None => {
                            debug!("SSE stream ended");
                            connected.store(false, Ordering::Relaxed);
                            break;
                        }
                    }
                }
            }
        }

        // Reconnect with backoff
        if !cancel.is_cancelled() {
            reconnects.fetch_add(1, Ordering::Relaxed);
            let delay = backoffs[backoff_idx.min(backoffs.len() - 1)];
            backoff_idx = (backoff_idx + 1).min(backoffs.len() - 1);
            debug!("SSE reconnecting in {:?}", delay);

            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = tokio::time::sleep(delay) => {}
            }
        }
    }
}
