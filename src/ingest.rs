use std::io::Write;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use flate2::write::GzEncoder;
use flate2::Compression;
use hmac::{Hmac, Mac};
use reqwest::header::HeaderValue;
use sha2::Sha256;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::types::{BatchPayload, ReportEntry};

const BATCH_SIZE: usize = 500;
const FLUSH_INTERVAL: Duration = Duration::from_secs(15);
const CHANNEL_CAPACITY: usize = 10_000;
const BACKOFF_SCHEDULE: &[Duration] = &[
    Duration::from_millis(100),
    Duration::from_millis(400),
    Duration::from_secs(1),
];

pub(crate) struct IngestStats {
    pub dropped_samples: Arc<AtomicU64>,
}

pub(crate) struct IngestHandle {
    pub tx: mpsc::Sender<ReportEntry>,
    pub stats: IngestStats,
    pub task_handle: tokio::task::JoinHandle<()>,
}

impl IngestHandle {
    pub fn buffer_capacity(&self) -> usize {
        CHANNEL_CAPACITY
    }
}

pub(crate) fn start_flusher(
    base_url: String,
    project_id: String,
    ingest_secret: Option<String>,
    api_key: String,
    cancel: CancellationToken,
) -> IngestHandle {
    let (tx, rx) = mpsc::channel::<ReportEntry>(CHANNEL_CAPACITY);
    let dropped = Arc::new(AtomicU64::new(0));
    let stats = IngestStats {
        dropped_samples: dropped.clone(),
    };

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("failed to build ingest HTTP client");

    let task_handle = tokio::spawn(flusher_loop(
        rx,
        http,
        base_url,
        project_id,
        ingest_secret,
        api_key,
        cancel,
    ));

    IngestHandle {
        tx,
        stats,
        task_handle,
    }
}

async fn flusher_loop(
    mut rx: mpsc::Receiver<ReportEntry>,
    http: reqwest::Client,
    base_url: String,
    project_id: String,
    ingest_secret: Option<String>,
    api_key: String,
    cancel: CancellationToken,
) {
    let url = format!("{base_url}/v1/projects/{project_id}/ingest");
    let mut buffer: Vec<ReportEntry> = Vec::with_capacity(BATCH_SIZE);
    let mut interval = tokio::time::interval(FLUSH_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                // Drain remaining messages
                while let Ok(entry) = rx.try_recv() {
                    buffer.push(entry);
                }
                if !buffer.is_empty() {
                    let batch = std::mem::take(&mut buffer);
                    send_batch(&http, &url, ingest_secret.as_deref(), &api_key, batch).await;
                }
                break;
            }
            entry = rx.recv() => {
                match entry {
                    Some(entry) => {
                        buffer.push(entry);
                        if buffer.len() >= BATCH_SIZE {
                            let batch = std::mem::take(&mut buffer);
                            send_batch(&http, &url, ingest_secret.as_deref(), &api_key, batch).await;
                        }
                    }
                    None => {
                        // Channel closed
                        if !buffer.is_empty() {
                            let batch = std::mem::take(&mut buffer);
                            send_batch(&http, &url, ingest_secret.as_deref(), &api_key, batch).await;
                        }
                        break;
                    }
                }
            }
            _ = interval.tick() => {
                if !buffer.is_empty() {
                    let batch = std::mem::take(&mut buffer);
                    send_batch(&http, &url, ingest_secret.as_deref(), &api_key, batch).await;
                }
            }
        }
    }
}

async fn send_batch(
    http: &reqwest::Client,
    url: &str,
    ingest_secret: Option<&str>,
    api_key: &str,
    samples: Vec<ReportEntry>,
) {
    let payload = BatchPayload { samples };
    let json_bytes = match serde_json::to_vec(&payload) {
        Ok(b) => b,
        Err(e) => {
            warn!("failed to serialize ingest payload: {e}");
            return;
        }
    };

    // Gzip compress
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    if let Err(e) = encoder.write_all(&json_bytes) {
        warn!("failed to gzip ingest payload: {e}");
        return;
    }
    let compressed = match encoder.finish() {
        Ok(b) => b,
        Err(e) => {
            warn!("failed to finish gzip: {e}");
            return;
        }
    };

    let ts_ms = chrono::Utc::now().timestamp_millis();

    for (attempt, backoff) in std::iter::once(&Duration::ZERO)
        .chain(BACKOFF_SCHEDULE.iter())
        .enumerate()
    {
        if attempt > 0 {
            tokio::time::sleep(*backoff).await;
            debug!("retrying ingest batch (attempt {attempt})");
        }

        let mut req = http
            .post(url)
            .header("Content-Type", "application/json")
            .header("Content-Encoding", "gzip")
            .header("Authorization", format!("Bearer {api_key}"))
            .header(
                "X-Contract-Version",
                HeaderValue::from_static(crate::CONTRACT_VERSION),
            )
            .header("X-EB-Timestamp", ts_ms.to_string())
            .body(compressed.clone());

        // HMAC signature if secret is configured
        if let Some(secret) = ingest_secret {
            if let Some(sig) = compute_signature(secret, ts_ms, &compressed) {
                req = req.header("X-EB-Signature", sig);
            }
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                if (200..300).contains(&status) {
                    debug!("ingest batch sent successfully ({} samples)", payload.samples.len());
                    return;
                }
                if status == 401 || status == 403 {
                    warn!("ingest auth error (HTTP {status}), not retrying");
                    return;
                }
                warn!("ingest request failed (HTTP {status})");
            }
            Err(e) => {
                warn!("ingest request transport error: {e}");
            }
        }
    }
}

pub(crate) fn compute_signature(secret_hex: &str, ts_ms: i64, compressed: &[u8]) -> Option<String> {
    let secret_bytes = match hex::decode(secret_hex) {
        Ok(b) => b,
        Err(e) => {
            warn!("failed to decode ingest secret hex: {e}");
            return None;
        }
    };

    let mut mac =
        Hmac::<Sha256>::new_from_slice(&secret_bytes).expect("HMAC can take key of any size");

    // Message: "{ts_ms}.{compressed_bytes}"
    let ts_str = ts_ms.to_string();
    mac.update(ts_str.as_bytes());
    mac.update(b".");
    mac.update(compressed);

    let result = mac.finalize();
    let sig_hex = hex::encode(result.into_bytes());
    Some(format!("v1={sig_hex}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // A valid 32-byte (64 hex char) secret
    const TEST_SECRET: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn compute_signature_valid_hex_produces_v1_prefixed_sig() {
        let sig = compute_signature(TEST_SECRET, 1700000000000, b"test payload").unwrap();
        assert!(sig.starts_with("v1="));
        // v1= + 64 hex chars = 67 chars total
        assert_eq!(sig.len(), 67);
    }

    #[test]
    fn compute_signature_deterministic() {
        let sig1 = compute_signature(TEST_SECRET, 1700000000000, b"hello").unwrap();
        let sig2 = compute_signature(TEST_SECRET, 1700000000000, b"hello").unwrap();
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn compute_signature_different_payloads_different_sigs() {
        let sig1 = compute_signature(TEST_SECRET, 1700000000000, b"payload-a").unwrap();
        let sig2 = compute_signature(TEST_SECRET, 1700000000000, b"payload-b").unwrap();
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn compute_signature_different_timestamps_different_sigs() {
        let sig1 = compute_signature(TEST_SECRET, 1000, b"same").unwrap();
        let sig2 = compute_signature(TEST_SECRET, 2000, b"same").unwrap();
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn compute_signature_invalid_hex_returns_none() {
        let sig = compute_signature("not-valid-hex-zzzz", 1000, b"test");
        assert!(sig.is_none());
    }

    #[test]
    fn compute_signature_empty_payload() {
        let sig = compute_signature(TEST_SECRET, 1000, b"");
        assert!(sig.is_some());
        assert!(sig.unwrap().starts_with("v1="));
    }

    #[test]
    fn compute_signature_short_secret() {
        // HMAC-SHA256 accepts any key length
        let sig = compute_signature("aabb", 1000, b"test");
        assert!(sig.is_some());
    }

    // ── ReportEntry / BatchPayload serialization ───────────────────

    #[test]
    fn report_entry_serializes_correctly() {
        let entry = ReportEntry {
            router_id: "r1".to_string(),
            metric: "latency".to_string(),
            ts_ms: 1700000000000,
            value: 42.5,
            ok: true,
            tags: None,
            trace_id: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["router_id"], "r1");
        assert_eq!(parsed["metric"], "latency");
        assert_eq!(parsed["value"], 42.5);
        assert_eq!(parsed["ok"], true);
    }

    #[test]
    fn batch_payload_wraps_samples() {
        let payload = BatchPayload {
            samples: vec![ReportEntry {
                router_id: "r1".to_string(),
                metric: "count".to_string(),
                ts_ms: 100,
                value: 1.0,
                ok: true,
                tags: None,
                trace_id: None,
            }],
        };
        let json = serde_json::to_string(&payload).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["samples"].as_array().unwrap().len(), 1);
    }
}
