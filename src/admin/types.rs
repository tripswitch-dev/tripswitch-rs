use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Enums ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreakerKind {
    Standard,
    Canary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreakerOp {
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HalfOpenPolicy {
    Probabilistic,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterMode {
    Random,
    RoundRobin,
    Hash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannelType {
    Slack,
    Email,
    Webhook,
    PagerDuty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationEventType {
    BreakerOpened,
    BreakerClosed,
    BreakerHalfOpened,
    BreakerCreated,
    BreakerDeleted,
}

pub use crate::types::BreakerStateValue;

// ── Domain Structs ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breaker {
    pub id: String,
    pub project_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub kind: BreakerKind,
    pub metric: String,
    pub threshold: f64,
    pub op: BreakerOp,
    pub window_size: i64,
    pub min_samples: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_policy: Option<HalfOpenPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_max_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakerState {
    pub breaker_id: String,
    pub breaker_name: String,
    pub state: BreakerStateValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_evaluated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Router {
    pub id: String,
    pub project_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub mode: RouterMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub breaker_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub channel_type: NotificationChannelType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    #[serde(default)]
    pub events: Vec<NotificationEventType>,
    #[serde(default)]
    pub breaker_ids: Vec<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub project_id: String,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breaker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breaker_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectKey {
    pub id: String,
    pub project_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectKeyResponse {
    pub key: ProjectKey,
    pub raw_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestSecretRotation {
    pub ingest_secret: String,
}

// ── Input Structs ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct CreateProjectInput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateProjectInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateBreakerInput {
    pub name: String,
    pub metric: String,
    pub threshold: f64,
    pub op: BreakerOp,
    pub window_size: i64,
    pub min_samples: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<BreakerKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_policy: Option<HalfOpenPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_max_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateBreakerInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op: Option<BreakerOp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_samples: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_policy: Option<HalfOpenPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub half_open_max_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateRouterInput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<RouterMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
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
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateNotificationChannelInput {
    pub name: String,
    pub channel_type: NotificationChannelType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<NotificationEventType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breaker_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateNotificationChannelInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<NotificationEventType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breaker_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateProjectKeyInput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateProjectKeyInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

// ── Helper / Special Structs ───────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SyncBreakersInput {
    pub breakers: Vec<CreateBreakerInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchGetBreakerStatesInput {
    pub breaker_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkBreakerInput {
    pub breaker_id: String,
}

// ── Pagination ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ListParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl ListParams {
    pub fn to_query_pairs(&self) -> Vec<(&str, String)> {
        let mut pairs = Vec::new();
        if let Some(page) = self.page {
            pairs.push(("page", page.to_string()));
        }
        if let Some(per_page) = self.per_page {
            pairs.push(("per_page", per_page.to_string()));
        }
        pairs
    }
}

#[derive(Debug, Clone, Default)]
pub struct ListEventsParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub breaker_id: Option<String>,
    pub event_type: Option<String>,
}

impl ListEventsParams {
    pub fn to_query_pairs(&self) -> Vec<(&str, String)> {
        let mut pairs = Vec::new();
        if let Some(page) = self.page {
            pairs.push(("page", page.to_string()));
        }
        if let Some(per_page) = self.per_page {
            pairs.push(("per_page", per_page.to_string()));
        }
        if let Some(ref breaker_id) = self.breaker_id {
            pairs.push(("breaker_id", breaker_id.clone()));
        }
        if let Some(ref event_type) = self.event_type {
            pairs.push(("event_type", event_type.clone()));
        }
        pairs
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub total_pages: i64,
}

pub type ListProjectsResponse = Page<Project>;
pub type ListBreakersResponse = Page<Breaker>;
pub type ListRoutersResponse = Page<Router>;
pub type ListNotificationChannelsResponse = Page<NotificationChannel>;
pub type ListEventsResponse = Page<Event>;
pub type ListProjectKeysResponse = Page<ProjectKey>;

#[cfg(test)]
mod tests {
    use super::*;

    // ── Enum serde round-trips ─────────────────────────────────────

    #[test]
    fn breaker_kind_serde() {
        assert_eq!(
            serde_json::to_string(&BreakerKind::Standard).unwrap(),
            r#""standard""#
        );
        assert_eq!(
            serde_json::to_string(&BreakerKind::Canary).unwrap(),
            r#""canary""#
        );
        let rt: BreakerKind = serde_json::from_str(r#""standard""#).unwrap();
        assert_eq!(rt, BreakerKind::Standard);
        let rt: BreakerKind = serde_json::from_str(r#""canary""#).unwrap();
        assert_eq!(rt, BreakerKind::Canary);
    }

    #[test]
    fn breaker_op_serde() {
        for (variant, expected) in [
            (BreakerOp::Gt, "gt"),
            (BreakerOp::Gte, "gte"),
            (BreakerOp::Lt, "lt"),
            (BreakerOp::Lte, "lte"),
            (BreakerOp::Eq, "eq"),
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
            (HalfOpenPolicy::Probabilistic, "probabilistic"),
            (HalfOpenPolicy::Disabled, "disabled"),
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
            (RouterMode::Random, "random"),
            (RouterMode::RoundRobin, "round_robin"),
            (RouterMode::Hash, "hash"),
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
            (NotificationChannelType::Email, "email"),
            (NotificationChannelType::Webhook, "webhook"),
            (NotificationChannelType::PagerDuty, "pager_duty"),
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
            (NotificationEventType::BreakerOpened, "breaker_opened"),
            (NotificationEventType::BreakerClosed, "breaker_closed"),
            (NotificationEventType::BreakerHalfOpened, "breaker_half_opened"),
            (NotificationEventType::BreakerCreated, "breaker_created"),
            (NotificationEventType::BreakerDeleted, "breaker_deleted"),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let rt: NotificationEventType = serde_json::from_str(&json).unwrap();
            assert_eq!(rt, variant);
        }
    }

    #[test]
    fn breaker_state_value_serde() {
        for (variant, expected) in [
            (BreakerStateValue::Open, "open"),
            (BreakerStateValue::Closed, "closed"),
            (BreakerStateValue::HalfOpen, "half_open"),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{expected}\""));
            let rt: BreakerStateValue = serde_json::from_str(&json).unwrap();
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
    fn list_params_partial() {
        let p = ListParams {
            page: Some(2),
            per_page: None,
        };
        let pairs = p.to_query_pairs();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], ("page", "2".to_string()));
    }

    #[test]
    fn list_params_full() {
        let p = ListParams {
            page: Some(3),
            per_page: Some(25),
        };
        let pairs = p.to_query_pairs();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("page", "3".to_string()));
        assert_eq!(pairs[1], ("per_page", "25".to_string()));
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

    #[test]
    fn list_events_params_full() {
        let p = ListEventsParams {
            page: Some(1),
            per_page: Some(10),
            breaker_id: Some("b_123".to_string()),
            event_type: Some("breaker_opened".to_string()),
        };
        let pairs = p.to_query_pairs();
        assert_eq!(pairs.len(), 4);
    }

    // ── Page<T> deserialization ─────────────────────────────────────

    #[test]
    fn page_deserialize() {
        let json = r#"{
            "data": [{"id":"p1","name":"Proj 1","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"}],
            "page": 1,
            "per_page": 10,
            "total": 1,
            "total_pages": 1
        }"#;
        let page: Page<Project> = serde_json::from_str(json).unwrap();
        assert_eq!(page.data.len(), 1);
        assert_eq!(page.page, 1);
        assert_eq!(page.per_page, 10);
        assert_eq!(page.total, 1);
        assert_eq!(page.total_pages, 1);
        assert_eq!(page.data[0].id, "p1");
    }

    // ── Input struct serialization (skip_serializing_if) ───────────

    #[test]
    fn create_breaker_input_required_fields_only() {
        let input = CreateBreakerInput {
            name: "api-latency".to_string(),
            metric: "p99_latency".to_string(),
            threshold: 500.0,
            op: BreakerOp::Gt,
            window_size: 300,
            min_samples: 100,
            kind: None,
            description: None,
            half_open_policy: None,
            half_open_max_rate: None,
            cooldown: None,
            metadata: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"name\":\"api-latency\""));
        assert!(json.contains("\"threshold\":500.0"));
        assert!(json.contains("\"op\":\"gt\""));
        // None fields should be omitted
        assert!(!json.contains("\"kind\""));
        assert!(!json.contains("\"description\""));
        assert!(!json.contains("\"metadata\""));
        assert!(!json.contains("\"half_open_policy\""));
    }

    #[test]
    fn create_breaker_input_with_metadata() {
        let input = CreateBreakerInput {
            name: "test".to_string(),
            metric: "latency".to_string(),
            threshold: 100.0,
            op: BreakerOp::Gte,
            window_size: 60,
            min_samples: 10,
            kind: Some(BreakerKind::Standard),
            description: None,
            half_open_policy: None,
            half_open_max_rate: None,
            cooldown: None,
            metadata: Some(serde_json::json!({"region": "us-east-1", "team": "payments"})),
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"metadata\""));
        assert!(json.contains("us-east-1"));
        assert!(json.contains("payments"));
    }

    #[test]
    fn update_breaker_input_single_field() {
        let input = UpdateBreakerInput {
            name: Some("new-name".to_string()),
            description: None,
            metric: None,
            threshold: None,
            op: None,
            window_size: None,
            min_samples: None,
            half_open_policy: None,
            half_open_max_rate: None,
            cooldown: None,
            metadata: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert_eq!(json, r#"{"name":"new-name"}"#);
    }

    #[test]
    fn create_project_input_without_description() {
        let input = CreateProjectInput {
            name: "my-project".to_string(),
            description: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert_eq!(json, r#"{"name":"my-project"}"#);
    }

    #[test]
    fn create_project_input_with_description() {
        let input = CreateProjectInput {
            name: "my-project".to_string(),
            description: Some("A test project".to_string()),
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"description\":\"A test project\""));
    }

    #[test]
    fn create_router_input_with_mode() {
        let input = CreateRouterInput {
            name: "my-router".to_string(),
            description: None,
            mode: Some(RouterMode::RoundRobin),
            metadata: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"mode\":\"round_robin\""));
        assert!(!json.contains("\"description\""));
    }

    #[test]
    fn create_notification_channel_input_with_channel_type() {
        let input = CreateNotificationChannelInput {
            name: "my-channel".to_string(),
            channel_type: NotificationChannelType::Slack,
            config: None,
            events: Some(vec![
                NotificationEventType::BreakerOpened,
                NotificationEventType::BreakerClosed,
            ]),
            breaker_ids: None,
            enabled: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"channel_type\":\"slack\""));
        assert!(json.contains("\"breaker_opened\""));
        assert!(json.contains("\"breaker_closed\""));
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
            description: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert_eq!(json, r#"{"name":"Updated Name"}"#);
    }
}
