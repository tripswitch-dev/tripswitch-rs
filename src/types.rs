use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The state of a single breaker as observed by the runtime client.
#[derive(Debug, Clone)]
pub struct BreakerStatus {
    pub name: String,
    pub state: BreakerStateValue,
    pub allow_rate: Option<f64>,
}

/// Possible breaker states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreakerStateValue {
    Open,
    Closed,
    HalfOpen,
}

/// Aggregate status of the project's breakers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub open_count: i64,
    pub closed_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_eval_ms: Option<i64>,
}

/// SDK statistics for monitoring the health of the runtime client.
#[derive(Debug, Clone)]
pub struct SdkStats {
    pub dropped_samples: u64,
    pub buffer_capacity: usize,
    pub sse_connected: bool,
    pub sse_reconnects: u64,
    pub flush_failures: u64,
    pub last_successful_flush: Option<chrono::DateTime<chrono::Utc>>,
    pub last_sse_event: Option<chrono::DateTime<chrono::Utc>>,
    pub cached_breakers: usize,
}

/// Breaker metadata from the metadata cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakerMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
}

/// Router metadata from the metadata cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
}

/// Metadata list responses from API.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BreakersMetadataResponse {
    pub breakers: Vec<BreakerMeta>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RoutersMetadataResponse {
    pub routers: Vec<RouterMeta>,
}

/// Input for the `report()` method.
pub struct ReportInput {
    pub router_id: String,
    pub metric: String,
    pub value: MetricValue,
    pub ok: bool,
    pub trace_id: Option<String>,
    pub tags: Option<HashMap<String, String>>,
}

/// Metric value types for execute/report.
pub enum MetricValue {
    /// Automatically use the task's latency in milliseconds.
    Latency,
    /// A static numeric value.
    Static(f64),
    /// A deferred value computed after the task executes.
    Dynamic(Box<dyn FnOnce() -> f64 + Send>),
}

impl std::fmt::Debug for MetricValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricValue::Latency => write!(f, "Latency"),
            MetricValue::Static(v) => write!(f, "Static({v})"),
            MetricValue::Dynamic(_) => write!(f, "Dynamic(<fn>)"),
        }
    }
}

// ── Internal wire types ────────────────────────────────────────────

/// A single sample entry sent to the ingest endpoint.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReportEntry {
    pub router_id: String,
    pub metric: String,
    pub ts_ms: i64,
    pub value: f64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

/// The batch payload POSTed to the ingest endpoint.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct BatchPayload {
    pub samples: Vec<ReportEntry>,
}

/// Breaker state entry from SSE events.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BreakerStateEntry {
    pub breaker: String,
    pub state: BreakerStateValue,
    pub allow_rate: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── BreakerStateValue serde round-trips ────────────────────────

    #[test]
    fn breaker_state_value_serialize() {
        assert_eq!(
            serde_json::to_string(&BreakerStateValue::Open).unwrap(),
            r#""open""#
        );
        assert_eq!(
            serde_json::to_string(&BreakerStateValue::Closed).unwrap(),
            r#""closed""#
        );
        assert_eq!(
            serde_json::to_string(&BreakerStateValue::HalfOpen).unwrap(),
            r#""half_open""#
        );
    }

    #[test]
    fn breaker_state_value_deserialize() {
        let open: BreakerStateValue = serde_json::from_str(r#""open""#).unwrap();
        assert_eq!(open, BreakerStateValue::Open);
        let closed: BreakerStateValue = serde_json::from_str(r#""closed""#).unwrap();
        assert_eq!(closed, BreakerStateValue::Closed);
        let half: BreakerStateValue = serde_json::from_str(r#""half_open""#).unwrap();
        assert_eq!(half, BreakerStateValue::HalfOpen);
    }

    // ── Status deserialization ─────────────────────────────────────

    #[test]
    fn status_deserialize_full() {
        let json = r#"{"open_count":2,"closed_count":8,"last_eval_ms":1234567890}"#;
        let status: Status = serde_json::from_str(json).unwrap();
        assert_eq!(status.open_count, 2);
        assert_eq!(status.closed_count, 8);
        assert_eq!(status.last_eval_ms, Some(1234567890));
    }

    #[test]
    fn status_deserialize_without_last_eval_ms() {
        let json = r#"{"open_count":0,"closed_count":5}"#;
        let status: Status = serde_json::from_str(json).unwrap();
        assert_eq!(status.open_count, 0);
        assert_eq!(status.closed_count, 5);
        assert!(status.last_eval_ms.is_none());
    }

    #[test]
    fn status_serialize_skips_none_last_eval_ms() {
        let status = Status {
            open_count: 1,
            closed_count: 3,
            last_eval_ms: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(!json.contains("last_eval_ms"));
    }

    // ── BreakerMeta deserialization ────────────────────────────────

    #[test]
    fn breaker_meta_deserialize_with_metadata() {
        let json = r#"{"id":"b1","name":"test-breaker","metadata":{"region":"us-east-1"}}"#;
        let meta: BreakerMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.id, "b1");
        assert_eq!(meta.name, "test-breaker");
        let md = meta.metadata.unwrap();
        assert_eq!(md.get("region").unwrap(), "us-east-1");
    }

    #[test]
    fn breaker_meta_deserialize_without_metadata() {
        let json = r#"{"id":"b1","name":"test-breaker"}"#;
        let meta: BreakerMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.id, "b1");
        assert!(meta.metadata.is_none());
    }

    // ── RouterMeta deserialization ─────────────────────────────────

    #[test]
    fn router_meta_deserialize_with_metadata() {
        let json = r#"{"id":"r1","name":"test-router","metadata":{"env":"production"}}"#;
        let meta: RouterMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.id, "r1");
        let md = meta.metadata.unwrap();
        assert_eq!(md.get("env").unwrap(), "production");
    }

    #[test]
    fn router_meta_deserialize_without_metadata() {
        let json = r#"{"id":"r1","name":"test-router"}"#;
        let meta: RouterMeta = serde_json::from_str(json).unwrap();
        assert!(meta.metadata.is_none());
    }

    // ── BreakerStateEntry deserialization (SSE format) ──────────────

    #[test]
    fn breaker_state_entry_deserialize() {
        let json = r#"{"breaker":"my-breaker","state":"open","allow_rate":null}"#;
        let entry: BreakerStateEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.breaker, "my-breaker");
        assert_eq!(entry.state, BreakerStateValue::Open);
        assert!(entry.allow_rate.is_none());
    }

    #[test]
    fn breaker_state_entry_deserialize_half_open_with_rate() {
        let json = r#"{"breaker":"hb","state":"half_open","allow_rate":0.3}"#;
        let entry: BreakerStateEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.state, BreakerStateValue::HalfOpen);
        assert_eq!(entry.allow_rate, Some(0.3));
    }

    #[test]
    fn breaker_state_entry_deserialize_closed() {
        let json = r#"{"breaker":"cb","state":"closed","allow_rate":1.0}"#;
        let entry: BreakerStateEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.state, BreakerStateValue::Closed);
        assert_eq!(entry.allow_rate, Some(1.0));
    }

    // ── BreakersMetadataResponse / RoutersMetadataResponse ─────────

    #[test]
    fn breakers_metadata_response_deserialize() {
        let json = r#"{"breakers":[{"id":"b1","name":"breaker-1"},{"id":"b2","name":"breaker-2","metadata":{"env":"prod"}}]}"#;
        let resp: BreakersMetadataResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.breakers.len(), 2);
        assert_eq!(resp.breakers[0].id, "b1");
        assert!(resp.breakers[0].metadata.is_none());
        assert!(resp.breakers[1].metadata.is_some());
    }

    #[test]
    fn routers_metadata_response_deserialize() {
        let json = r#"{"routers":[{"id":"r1","name":"router-1"}]}"#;
        let resp: RoutersMetadataResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.routers.len(), 1);
        assert_eq!(resp.routers[0].name, "router-1");
    }

    // ── ReportEntry serialization ──────────────────────────────────

    #[test]
    fn report_entry_serialize_full() {
        let entry = ReportEntry {
            router_id: "r1".to_string(),
            metric: "latency".to_string(),
            ts_ms: 1700000000000,
            value: 42.5,
            ok: true,
            tags: Some(HashMap::from([("env".to_string(), "prod".to_string())])),
            trace_id: Some("trace_123".to_string()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"router_id\":\"r1\""));
        assert!(json.contains("\"metric\":\"latency\""));
        assert!(json.contains("\"ts_ms\":1700000000000"));
        assert!(json.contains("\"value\":42.5"));
        assert!(json.contains("\"ok\":true"));
        assert!(json.contains("\"trace_id\":\"trace_123\""));
        assert!(json.contains("\"env\":\"prod\""));
    }

    #[test]
    fn report_entry_serialize_skips_none_tags_and_trace_id() {
        let entry = ReportEntry {
            router_id: "r1".to_string(),
            metric: "count".to_string(),
            ts_ms: 1700000000000,
            value: 1.0,
            ok: false,
            tags: None,
            trace_id: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(!json.contains("tags"));
        assert!(!json.contains("trace_id"));
    }

    // ── BatchPayload serialization ─────────────────────────────────

    #[test]
    fn batch_payload_serialize() {
        let payload = BatchPayload {
            samples: vec![
                ReportEntry {
                    router_id: "r1".to_string(),
                    metric: "latency".to_string(),
                    ts_ms: 100,
                    value: 10.0,
                    ok: true,
                    tags: None,
                    trace_id: None,
                },
                ReportEntry {
                    router_id: "r1".to_string(),
                    metric: "count".to_string(),
                    ts_ms: 100,
                    value: 1.0,
                    ok: true,
                    tags: None,
                    trace_id: None,
                },
            ],
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"samples\":["));
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["samples"].as_array().unwrap().len(), 2);
    }

    // ── MetricValue Debug ──────────────────────────────────────────

    #[test]
    fn metric_value_debug() {
        assert_eq!(format!("{:?}", MetricValue::Latency), "Latency");
        assert_eq!(format!("{:?}", MetricValue::Static(42.0)), "Static(42)");
        assert_eq!(
            format!("{:?}", MetricValue::Dynamic(Box::new(|| 1.0))),
            "Dynamic(<fn>)"
        );
    }
}
