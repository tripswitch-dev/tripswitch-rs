use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Enums ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreakerKind {
    ErrorRate,
    Avg,
    P95,
    Max,
    Min,
    Sum,
    Stddev,
    Count,
    Percentile,
    ConsecutiveFailures,
    Delta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreakerOp {
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HalfOpenPolicy {
    Optimistic,
    Conservative,
    Pessimistic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterMode {
    Static,
    Canary,
    Weighted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannelType {
    Slack,
    #[serde(rename = "pagerduty")]
    PagerDuty,
    Email,
    Webhook,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationEventType {
    Trip,
    Recover,
}

// ── Domain Structs ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    #[serde(rename = "project_id")]
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slack_webhook_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id_url_template: Option<String>,
    #[serde(default)]
    pub enable_signed_ingest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breaker {
    pub id: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub router_ids: Vec<String>,
    pub name: String,
    pub metric: String,
    pub kind: BreakerKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind_params: Option<HashMap<String, serde_json::Value>>,
    pub op: BreakerOp,
    pub threshold: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_state_duration_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_interval_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_confirmation_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_backoff_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_backoff_cap_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_indeterminate_policy: Option<HalfOpenPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_window_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_allow_rate_ramp_steps: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakerState {
    pub breaker_id: String,
    pub state: String,
    pub allow_rate: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Router {
    pub id: String,
    pub name: String,
    pub mode: RouterMode,
    #[serde(default)]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breaker_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakers: Option<Vec<Breaker>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inserted_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub channel: NotificationChannelType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub events: Vec<NotificationEventType>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub project_id: String,
    pub breaker_id: String,
    pub from_state: String,
    pub to_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectKey {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inserted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectKeyResponse {
    pub id: String,
    pub name: String,
    pub key: String,
    pub key_prefix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestSecretRotation {
    pub ingest_secret: String,
}

// ── Input Structs ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct CreateProjectInput {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateProjectInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slack_webhook_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id_url_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_signed_ingest: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateBreakerInput {
    pub name: String,
    pub metric: String,
    pub kind: BreakerKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind_params: Option<HashMap<String, serde_json::Value>>,
    pub op: BreakerOp,
    pub threshold: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_state_duration_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_interval_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_backoff_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_backoff_cap_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_indeterminate_policy: Option<HalfOpenPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_allow_rate_ramp_steps: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateBreakerInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<BreakerKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind_params: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op: Option<BreakerOp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_state_duration_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_interval_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_backoff_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_backoff_cap_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_indeterminate_policy: Option<HalfOpenPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_allow_rate_ramp_steps: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateRouterInput {
    pub name: String,
    pub mode: RouterMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateRouterInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<RouterMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateNotificationChannelInput {
    pub name: String,
    pub channel: NotificationChannelType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<NotificationEventType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateNotificationChannelInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<NotificationEventType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateProjectKeyInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

// ── Helper / Special Structs ───────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SyncBreakersInput {
    pub breakers: Vec<CreateBreakerInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchGetBreakerStatesInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breaker_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub router_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkBreakerInput {
    pub breaker_id: String,
}

// ── Workspaces ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub org_id: String,
    pub inserted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateWorkspaceInput {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateWorkspaceInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListWorkspacesResponse {
    pub workspaces: Vec<Workspace>,
}

// ── Response Wrappers ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectsResponse {
    pub projects: Vec<Project>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBreakersResponse {
    pub breakers: Vec<Breaker>,
    pub count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRoutersResponse {
    pub routers: Vec<Router>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListEventsResponse {
    pub events: Vec<Event>,
    pub returned: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListNotificationChannelsResponse {
    pub items: Vec<NotificationChannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProjectKeysResponse {
    pub keys: Vec<ProjectKey>,
    pub count: i64,
}

// ── Pagination ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ListParams {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

impl ListParams {
    pub fn to_query_pairs(&self) -> Vec<(&str, String)> {
        let mut pairs = Vec::new();
        if let Some(ref cursor) = self.cursor {
            pairs.push(("cursor", cursor.clone()));
        }
        if let Some(limit) = self.limit {
            pairs.push(("limit", limit.to_string()));
        }
        pairs
    }
}

#[derive(Debug, Clone, Default)]
pub struct ListEventsParams {
    pub breaker_id: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

impl ListEventsParams {
    pub fn to_query_pairs(&self) -> Vec<(&str, String)> {
        let mut pairs = Vec::new();
        if let Some(ref breaker_id) = self.breaker_id {
            pairs.push(("breaker_id", breaker_id.clone()));
        }
        if let Some(ref start_time) = self.start_time {
            pairs.push(("start_time", start_time.to_rfc3339()));
        }
        if let Some(ref end_time) = self.end_time {
            pairs.push(("end_time", end_time.to_rfc3339()));
        }
        if let Some(ref cursor) = self.cursor {
            pairs.push(("cursor", cursor.clone()));
        }
        if let Some(limit) = self.limit {
            pairs.push(("limit", limit.to_string()));
        }
        pairs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Enum serde round-trips ─────────────────────────────────────

    #[test]
    fn breaker_kind_serde() {
        for (variant, expected) in [
            (BreakerKind::ErrorRate, "error_rate"),
            (BreakerKind::Avg, "avg"),
            (BreakerKind::P95, "p95"),
            (BreakerKind::Max, "max"),
            (BreakerKind::Min, "min"),
            (BreakerKind::Sum, "sum"),
            (BreakerKind::Stddev, "stddev"),
            (BreakerKind::Count, "count"),
            (BreakerKind::Percentile, "percentile"),
            (BreakerKind::ConsecutiveFailures, "consecutive_failures"),
            (BreakerKind::Delta, "delta"),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let rt: BreakerKind = serde_json::from_str(&json).unwrap();
            assert_eq!(rt, variant);
        }
    }

    #[test]
    fn breaker_op_serde() {
        for (variant, expected) in [
            (BreakerOp::Gt, "gt"),
            (BreakerOp::Gte, "gte"),
            (BreakerOp::Lt, "lt"),
            (BreakerOp::Lte, "lte"),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let rt: BreakerOp = serde_json::from_str(&json).unwrap();
            assert_eq!(rt, variant);
        }
    }

    #[test]
    fn half_open_policy_serde() {
        for (variant, expected) in [
            (HalfOpenPolicy::Optimistic, "optimistic"),
            (HalfOpenPolicy::Conservative, "conservative"),
            (HalfOpenPolicy::Pessimistic, "pessimistic"),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let rt: HalfOpenPolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(rt, variant);
        }
    }

    #[test]
    fn router_mode_serde() {
        for (variant, expected) in [
            (RouterMode::Static, "static"),
            (RouterMode::Canary, "canary"),
            (RouterMode::Weighted, "weighted"),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let rt: RouterMode = serde_json::from_str(&json).unwrap();
            assert_eq!(rt, variant);
        }
    }

    #[test]
    fn notification_channel_type_serde() {
        for (variant, expected) in [
            (NotificationChannelType::Slack, "slack"),
            (NotificationChannelType::PagerDuty, "pagerduty"),
            (NotificationChannelType::Email, "email"),
            (NotificationChannelType::Webhook, "webhook"),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let rt: NotificationChannelType = serde_json::from_str(&json).unwrap();
            assert_eq!(rt, variant);
        }
    }

    #[test]
    fn notification_event_type_serde() {
        for (variant, expected) in [
            (NotificationEventType::Trip, "trip"),
            (NotificationEventType::Recover, "recover"),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let rt: NotificationEventType = serde_json::from_str(&json).unwrap();
            assert_eq!(rt, variant);
        }
    }

    // ── ListParams::to_query_pairs ─────────────────────────────────

    #[test]
    fn list_params_empty() {
        let p = ListParams::default();
        assert!(p.to_query_pairs().is_empty());
    }

    #[test]
    fn list_params_with_cursor() {
        let p = ListParams {
            cursor: Some("abc123".to_string()),
            limit: None,
        };
        let pairs = p.to_query_pairs();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("cursor", "abc123".to_string()));
    }

    #[test]
    fn list_params_full() {
        let p = ListParams {
            cursor: Some("abc".to_string()),
            limit: Some(25),
        };
        let pairs = p.to_query_pairs();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("cursor", "abc".to_string()));
        assert_eq!(pairs[1], ("limit", "25".to_string()));
    }

    // ── ListEventsParams::to_query_pairs ───────────────────────────

    #[test]
    fn list_events_params_empty() {
        let p = ListEventsParams::default();
        assert!(p.to_query_pairs().is_empty());
    }

    #[test]
    fn list_events_params_partial() {
        let p = ListEventsParams {
            breaker_id: Some("b_123".to_string()),
            ..Default::default()
        };
        let pairs = p.to_query_pairs();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("breaker_id", "b_123".to_string()));
    }

    // ── Response deserialization ───────────────────────────────────

    #[test]
    fn list_projects_response_deserialize() {
        let json =
            r#"{"projects":[{"project_id":"p1","name":"Proj 1","enable_signed_ingest":false}]}"#;
        let resp: ListProjectsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.projects.len(), 1);
        assert_eq!(resp.projects[0].id, "p1");
    }

    #[test]
    fn list_breakers_response_deserialize() {
        let json = r#"{"breakers":[{"id":"b1","name":"test","metric":"latency","kind":"error_rate","op":"gt","threshold":0.5}],"count":1}"#;
        let resp: ListBreakersResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.breakers.len(), 1);
        assert_eq!(resp.count, 1);
    }

    #[test]
    fn list_events_response_deserialize() {
        let json = r#"{"events":[{"id":"e1","project_id":"p1","breaker_id":"b1","from_state":"closed","to_state":"open"}],"returned":1,"next_cursor":"abc"}"#;
        let resp: ListEventsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.events.len(), 1);
        assert_eq!(resp.returned, 1);
        assert_eq!(resp.next_cursor.as_deref(), Some("abc"));
    }

    // ── Input struct serialization ─────────────────────────────────

    #[test]
    fn create_breaker_input_required_fields_only() {
        let input = CreateBreakerInput {
            name: "api-latency".to_string(),
            metric: "p99_latency".to_string(),
            kind: BreakerKind::ErrorRate,
            kind_params: None,
            threshold: 500.0,
            op: BreakerOp::Gt,
            window_ms: None,
            min_count: None,
            min_state_duration_ms: None,
            cooldown_ms: None,
            eval_interval_ms: None,
            half_open_backoff_enabled: None,
            half_open_backoff_cap_ms: None,
            half_open_indeterminate_policy: None,
            recovery_allow_rate_ramp_steps: None,
            actions: None,
            metadata: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"name\":\"api-latency\""));
        assert!(json.contains("\"kind\":\"error_rate\""));
        assert!(json.contains("\"threshold\":500.0"));
        assert!(json.contains("\"op\":\"gt\""));
        assert!(!json.contains("\"window_ms\""));
        assert!(!json.contains("\"metadata\""));
    }

    #[test]
    fn create_breaker_input_with_metadata() {
        let input = CreateBreakerInput {
            name: "test".to_string(),
            metric: "latency".to_string(),
            kind: BreakerKind::Avg,
            kind_params: None,
            threshold: 100.0,
            op: BreakerOp::Gte,
            window_ms: Some(60000),
            min_count: Some(10),
            min_state_duration_ms: None,
            cooldown_ms: None,
            eval_interval_ms: None,
            half_open_backoff_enabled: None,
            half_open_backoff_cap_ms: None,
            half_open_indeterminate_policy: None,
            recovery_allow_rate_ramp_steps: None,
            actions: None,
            metadata: Some(HashMap::from([(
                "region".to_string(),
                "us-east-1".to_string(),
            )])),
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"metadata\""));
        assert!(json.contains("us-east-1"));
    }

    #[test]
    fn update_breaker_input_single_field() {
        let input = UpdateBreakerInput {
            name: Some("new-name".to_string()),
            metric: None,
            kind: None,
            kind_params: None,
            threshold: None,
            op: None,
            window_ms: None,
            min_count: None,
            min_state_duration_ms: None,
            cooldown_ms: None,
            eval_interval_ms: None,
            half_open_backoff_enabled: None,
            half_open_backoff_cap_ms: None,
            half_open_indeterminate_policy: None,
            recovery_allow_rate_ramp_steps: None,
            actions: None,
            metadata: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert_eq!(json, r#"{"name":"new-name"}"#);
    }

    #[test]
    fn create_project_input_serialization() {
        let input = CreateProjectInput {
            name: "my-project".to_string(),
        };
        let json = serde_json::to_string(&input).unwrap();
        assert_eq!(json, r#"{"name":"my-project"}"#);
    }

    #[test]
    fn create_router_input_with_mode() {
        let input = CreateRouterInput {
            name: "my-router".to_string(),
            mode: RouterMode::Static,
            description: None,
            enabled: None,
            metadata: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"mode\":\"static\""));
        assert!(!json.contains("\"description\""));
    }

    #[test]
    fn create_notification_channel_input_with_channel() {
        let input = CreateNotificationChannelInput {
            name: "my-channel".to_string(),
            channel: NotificationChannelType::Slack,
            config: None,
            events: Some(vec![
                NotificationEventType::Trip,
                NotificationEventType::Recover,
            ]),
            enabled: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"channel\":\"slack\""));
        assert!(json.contains("\"trip\""));
        assert!(json.contains("\"recover\""));
    }

    #[test]
    fn link_breaker_input_serialization() {
        let input = LinkBreakerInput {
            breaker_id: "breaker_456".to_string(),
        };
        let json = serde_json::to_string(&input).unwrap();
        assert_eq!(json, r#"{"breaker_id":"breaker_456"}"#);
    }

    #[test]
    fn update_project_input_single_field() {
        let input = UpdateProjectInput {
            name: Some("Updated Name".to_string()),
            slack_webhook_url: None,
            trace_id_url_template: None,
            enable_signed_ingest: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert_eq!(json, r#"{"name":"Updated Name"}"#);
    }

    #[test]
    fn project_deserialize_with_rename() {
        let json = r#"{"project_id":"p1","name":"Test","enable_signed_ingest":true}"#;
        let project: Project = serde_json::from_str(json).unwrap();
        assert_eq!(project.id, "p1");
        assert_eq!(project.name, "Test");
        assert!(project.enable_signed_ingest);
    }
}
