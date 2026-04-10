pub mod errors;
pub mod pager;
pub mod types;

mod breakers;
mod events;
mod notifications;
mod pagers;
mod project_keys;
mod projects;
mod routers;
mod workspaces;

use errors::{AdminError, ApiError};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;

const DEFAULT_BASE_URL: &str = "https://api.tripswitch.dev";

/// Options for individual admin requests.
#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    pub idempotency_key: Option<String>,
    pub timeout: Option<std::time::Duration>,
    pub request_id: Option<String>,
    pub headers: Option<HeaderMap>,
}

/// Builder for constructing an [`AdminClient`].
pub struct AdminClientBuilder {
    api_key: String,
    base_url: String,
    http_client: Option<reqwest::Client>,
}

impl AdminClientBuilder {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            http_client: None,
        }
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    pub fn build(self) -> AdminClient {
        let http = self.http_client.unwrap_or_else(|| {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client")
        });

        AdminClient {
            api_key: self.api_key,
            base_url: self.base_url.trim_end_matches('/').to_string(),
            http,
        }
    }
}

/// Admin client for managing Tripswitch resources (projects, breakers, routers, etc.).
#[derive(Clone)]
pub struct AdminClient {
    api_key: String,
    base_url: String,
    http: reqwest::Client,
}

impl AdminClient {
    pub fn builder(api_key: impl Into<String>) -> AdminClientBuilder {
        AdminClientBuilder::new(api_key)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let val = format!("Bearer {}", self.api_key);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_str(&val).expect("invalid api key characters"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "X-Contract-Version",
            HeaderValue::from_static(crate::CONTRACT_VERSION),
        );
        headers
    }

    fn apply_opts(
        &self,
        mut builder: reqwest::RequestBuilder,
        opts: Option<&RequestOptions>,
    ) -> reqwest::RequestBuilder {
        if let Some(opts) = opts {
            if let Some(ref key) = opts.idempotency_key {
                builder = builder.header("Idempotency-Key", key.as_str());
            }
            if let Some(ref id) = opts.request_id {
                builder = builder.header("X-Request-ID", id.as_str());
            }
            if let Some(timeout) = opts.timeout {
                builder = builder.timeout(timeout);
            }
            if let Some(ref extra) = opts.headers {
                for (k, v) in extra.iter() {
                    builder = builder.header(k, v);
                }
            }
        }
        builder
    }

    async fn do_request<T: DeserializeOwned>(
        &self,
        builder: reqwest::RequestBuilder,
        opts: Option<&RequestOptions>,
    ) -> Result<T, AdminError> {
        let builder = self.apply_opts(builder, opts);
        let resp = builder.send().await?;
        let status = resp.status().as_u16();

        if (200..300).contains(&status) {
            let body = resp.json::<T>().await?;
            return Ok(body);
        }

        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());

        let body_text = resp.text().await.unwrap_or_default();
        let mut api_err =
            serde_json::from_str::<ApiError>(&body_text).unwrap_or_else(|_| ApiError {
                status,
                code: String::new(),
                message: body_text.clone(),
                request_id: None,
                body: None,
                retry_after: None,
            });
        api_err.status = status;
        api_err.body = Some(body_text);
        api_err.retry_after = retry_after;

        Err(AdminError::from_api_error(api_err))
    }

    async fn do_request_no_content(
        &self,
        builder: reqwest::RequestBuilder,
        opts: Option<&RequestOptions>,
    ) -> Result<(), AdminError> {
        let builder = self.apply_opts(builder, opts);
        let resp = builder.send().await?;
        let status = resp.status().as_u16();

        if (200..300).contains(&status) {
            return Ok(());
        }

        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());

        let body_text = resp.text().await.unwrap_or_default();
        let mut api_err =
            serde_json::from_str::<ApiError>(&body_text).unwrap_or_else(|_| ApiError {
                status,
                code: String::new(),
                message: body_text.clone(),
                request_id: None,
                body: None,
                retry_after: None,
            });
        api_err.status = status;
        api_err.body = Some(body_text);
        api_err.retry_after = retry_after;

        Err(AdminError::from_api_error(api_err))
    }
}

#[cfg(test)]
mod tests {
    use super::types::*;
    use super::*;
    use httpmock::prelude::*;
    use serde_json::json;

    fn test_client(server: &MockServer) -> AdminClient {
        AdminClient::builder("eb_admin_test")
            .base_url(server.url(""))
            .build()
    }

    // ── AdminClientBuilder ─────────────────────────────────────────

    #[test]
    fn builder_default_base_url() {
        let client = AdminClient::builder("eb_admin_test").build();
        assert_eq!(client.url("/v1/test"), "https://api.tripswitch.dev/v1/test");
    }

    #[test]
    fn builder_custom_base_url() {
        let client = AdminClient::builder("eb_admin_test")
            .base_url("https://custom.api.dev")
            .build();
        assert_eq!(client.url("/v1/test"), "https://custom.api.dev/v1/test");
    }

    #[test]
    fn builder_trailing_slash_stripped() {
        let client = AdminClient::builder("eb_admin_test")
            .base_url("https://api.example.com/")
            .build();
        assert_eq!(
            client.url("/v1/projects"),
            "https://api.example.com/v1/projects"
        );
    }

    #[test]
    fn auth_headers_contains_required() {
        let client = AdminClient::builder("eb_admin_test").build();
        let headers = client.auth_headers();
        assert_eq!(
            headers.get("Authorization").unwrap().to_str().unwrap(),
            "Bearer eb_admin_test"
        );
        assert_eq!(
            headers.get("Content-Type").unwrap().to_str().unwrap(),
            "application/json"
        );
        assert_eq!(
            headers.get("X-Contract-Version").unwrap().to_str().unwrap(),
            crate::CONTRACT_VERSION
        );
    }

    #[test]
    fn request_options_default_all_none() {
        let opts = RequestOptions::default();
        assert!(opts.idempotency_key.is_none());
        assert!(opts.timeout.is_none());
        assert!(opts.request_id.is_none());
        assert!(opts.headers.is_none());
    }

    // ── CRUD Mock HTTP Tests ───────────────────────────────────────

    #[tokio::test]
    async fn get_project_sends_correct_request() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/v1/projects/proj_123")
                .header("Authorization", "Bearer eb_admin_test");
            then.status(200).json_body(json!({
                "project_id": "proj_123",
                "name": "My Project",
                "enable_signed_ingest": false
            }));
        });

        let client = test_client(&server);
        let project = client.get_project("proj_123").await.unwrap();

        mock.assert();
        assert_eq!(project.id, "proj_123");
        assert_eq!(project.name.as_deref(), Some("My Project"));
    }

    #[tokio::test]
    async fn list_projects_sends_get() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/v1/projects");
            then.status(200).json_body(json!({
                "projects": [
                    {"project_id":"p1","name":"Proj 1","enable_signed_ingest":false},
                    {"project_id":"p2","name":"Proj 2","enable_signed_ingest":false}
                ]
            }));
        });

        let client = test_client(&server);
        let resp = client.list_projects().await.unwrap();

        mock.assert();
        assert_eq!(resp.projects.len(), 2);
    }

    #[tokio::test]
    async fn create_project_sends_post() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/projects")
                .json_body(json!({"name": "my-project"}));
            then.status(201).json_body(json!({
                "project_id": "proj_new",
                "name": "my-project",
                "enable_signed_ingest": false
            }));
        });

        let client = test_client(&server);
        let input = CreateProjectInput {
            name: "my-project".to_string(),
            workspace_id: None,
        };
        let project = client.create_project(&input).await.unwrap();

        mock.assert();
        assert_eq!(project.name.as_deref(), Some("my-project"));
    }

    #[tokio::test]
    async fn update_project_sends_patch() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(PATCH)
                .path("/v1/projects/proj_123")
                .json_body(json!({"name": "Updated Name"}));
            then.status(200).json_body(json!({
                "project_id": "proj_123",
                "name": "Updated Name",
                "enable_signed_ingest": false
            }));
        });

        let client = test_client(&server);
        let input = UpdateProjectInput {
            name: Some("Updated Name".to_string()),
            slack_webhook_url: None,
            trace_id_url_template: None,
            enable_signed_ingest: None,
        };
        let project = client.update_project("proj_123", &input).await.unwrap();

        mock.assert();
        assert_eq!(project.name.as_deref(), Some("Updated Name"));
    }

    #[tokio::test]
    async fn delete_project_sends_delete_with_confirm_name() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(DELETE)
                .path("/v1/projects/proj_123")
                .json_body(json!({"confirm_name": "My Project"}));
            then.status(204);
        });

        let client = test_client(&server);
        client
            .delete_project("proj_123", "My Project")
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn create_breaker_sends_post_with_body() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/projects/proj_123/breakers")
                .header("Authorization", "Bearer eb_admin_test");
            then.status(201).json_body(json!({
                "breaker": {
                    "id": "breaker_456",
                    "name": "api-latency",
                    "kind": "error_rate",
                    "metric": "p99_latency",
                    "threshold": 500.0,
                    "op": "gt",
                    "window_ms": 300000,
                    "min_count": 100
                }
            }));
        });

        let client = test_client(&server);
        let input = CreateBreakerInput {
            name: "api-latency".to_string(),
            metric: "p99_latency".to_string(),
            kind: BreakerKind::ErrorRate,
            kind_params: None,
            threshold: 500.0,
            op: BreakerOp::Gt,
            window_ms: Some(300000),
            min_count: Some(100),
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
        let breaker = client.create_breaker("proj_123", &input).await.unwrap();

        mock.assert();
        assert_eq!(breaker.id, "breaker_456");
        assert_eq!(breaker.name, "api-latency");
    }

    #[tokio::test]
    async fn list_breakers_returns_response() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/v1/projects/proj_123/breakers");
            then.status(200).json_body(json!({
                "breakers": [
                    {
                        "id": "b1", "name": "breaker-1",
                        "kind": "error_rate", "metric": "latency", "threshold": 100.0,
                        "op": "gt"
                    }
                ],
                "count": 1
            }));
        });

        let client = test_client(&server);
        let resp = client.list_breakers("proj_123", None).await.unwrap();

        mock.assert();
        assert_eq!(resp.breakers.len(), 1);
        assert_eq!(resp.count, 1);
    }

    #[tokio::test]
    async fn delete_breaker_204_no_content() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(DELETE)
                .path("/v1/projects/proj_123/breakers/breaker_456");
            then.status(204);
        });

        let client = test_client(&server);
        client
            .delete_breaker("proj_123", "breaker_456")
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn create_router_sends_post_with_mode() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/projects/proj_123/routers")
                .json_body(json!({"name": "my-router", "mode": "static"}));
            then.status(201).json_body(json!({
                "id": "router_789",
                "name": "my-router",
                "mode": "static",
                "enabled": true
            }));
        });

        let client = test_client(&server);
        let input = CreateRouterInput {
            name: "my-router".to_string(),
            mode: RouterMode::Static,
            description: None,
            enabled: None,
            metadata: None,
        };
        let router = client.create_router("proj_123", &input).await.unwrap();

        mock.assert();
        assert_eq!(router.id, "router_789");
        assert_eq!(router.mode, RouterMode::Static);
    }

    #[tokio::test]
    async fn link_breaker_sends_post_with_breaker_id() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/projects/proj_123/routers/router_789/breakers")
                .json_body(json!({"breaker_id": "breaker_456"}));
            then.status(204);
        });

        let client = test_client(&server);
        let input = LinkBreakerInput {
            breaker_id: "breaker_456".to_string(),
        };
        client
            .link_breaker("proj_123", "router_789", &input)
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn create_notification_channel_with_type() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/projects/proj_123/notification-channels");
            then.status(201).json_body(json!({
                "id": "nc_1",
                "project_id": "proj_123",
                "name": "slack-alerts",
                "channel": "slack",
                "events": ["trip", "recover"],
                "enabled": true,
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            }));
        });

        let client = test_client(&server);
        let input = CreateNotificationChannelInput {
            name: "slack-alerts".to_string(),
            channel: NotificationChannelType::Slack,
            config: None,
            events: Some(vec![
                NotificationEventType::Trip,
                NotificationEventType::Recover,
            ]),
            enabled: None,
        };
        let channel = client
            .create_notification_channel("proj_123", &input)
            .await
            .unwrap();

        mock.assert();
        assert_eq!(channel.channel, NotificationChannelType::Slack);
        assert_eq!(channel.events.len(), 2);
    }

    #[tokio::test]
    async fn list_events_with_breaker_id_filter() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/v1/projects/proj_123/events")
                .query_param("breaker_id", "b_456");
            then.status(200).json_body(json!({
                "events": [
                    {
                        "id": "ev_1", "project_id": "proj_123",
                        "breaker_id": "b_456",
                        "from_state": "closed",
                        "to_state": "open",
                        "timestamp": "2024-01-01T00:00:00Z"
                    },
                    {
                        "id": "ev_2", "project_id": "proj_123",
                        "breaker_id": "b_456",
                        "from_state": "open",
                        "to_state": "closed",
                        "timestamp": "2024-01-02T00:00:00Z"
                    }
                ],
                "returned": 2
            }));
        });

        let client = test_client(&server);
        let params = ListEventsParams {
            breaker_id: Some("b_456".to_string()),
            ..Default::default()
        };
        let resp = client.list_events("proj_123", Some(&params)).await.unwrap();

        mock.assert();
        assert_eq!(resp.events.len(), 2);
        assert_eq!(resp.returned, 2);
    }

    // ── Error Classification with Mock Servers ─────────────────────

    #[tokio::test]
    async fn error_404_not_found() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/v1/projects/missing");
            then.status(404).json_body(json!({
                "status": 404,
                "code": "not_found",
                "message": "project not found",
                "request_id": "req_123"
            }));
        });

        let client = test_client(&server);
        let err = client.get_project("missing").await.unwrap_err();

        assert!(err.is_not_found());
        let api_err = err.api_error().unwrap();
        assert_eq!(api_err.status, 404);
        assert_eq!(api_err.code, "not_found");
        assert_eq!(api_err.request_id.as_deref(), Some("req_123"));
    }

    #[tokio::test]
    async fn error_429_rate_limited_with_retry_after() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/v1/projects/proj_123");
            then.status(429)
                .header("retry-after", "30")
                .json_body(json!({
                    "code": "rate_limited",
                    "message": "too many requests"
                }));
        });

        let client = test_client(&server);
        let err = client.get_project("proj_123").await.unwrap_err();

        assert!(err.is_rate_limited());
        let api_err = err.api_error().unwrap();
        assert_eq!(api_err.retry_after, Some(30));
    }

    #[tokio::test]
    async fn error_401_unauthorized() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/v1/projects/proj_123");
            then.status(401).json_body(json!({
                "code": "unauthorized",
                "message": "invalid api key"
            }));
        });

        let client = test_client(&server);
        let err = client.get_project("proj_123").await.unwrap_err();
        assert!(err.is_unauthorized());
    }

    #[tokio::test]
    async fn error_409_conflict() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/projects");
            then.status(409).json_body(json!({
                "code": "conflict",
                "message": "project already exists"
            }));
        });

        let client = test_client(&server);
        let input = CreateProjectInput {
            name: "dup".to_string(),
            workspace_id: None,
        };
        let err = client.create_project(&input).await.unwrap_err();
        assert!(err.is_conflict());
    }

    #[tokio::test]
    async fn error_422_validation() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/v1/projects");
            then.status(422).json_body(json!({
                "code": "validation_error",
                "message": "name is required"
            }));
        });

        let client = test_client(&server);
        let input = CreateProjectInput {
            name: "".to_string(),
            workspace_id: None,
        };
        let err = client.create_project(&input).await.unwrap_err();
        assert!(err.is_validation());
    }

    #[tokio::test]
    async fn error_500_server_fault() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/v1/projects/proj_123");
            then.status(500).json_body(json!({
                "code": "internal_error",
                "message": "something went wrong"
            }));
        });

        let client = test_client(&server);
        let err = client.get_project("proj_123").await.unwrap_err();
        assert!(err.is_server_fault());
    }

    #[tokio::test]
    async fn error_transport_connection_refused() {
        let client = AdminClient::builder("eb_admin_test")
            .base_url("http://127.0.0.1:1")
            .build();
        let err = client.get_project("proj_123").await.unwrap_err();
        assert!(err.is_transport());
    }

    // ── Request Options ────────────────────────────────────────────

    #[tokio::test]
    async fn request_options_idempotency_key_and_request_id() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/v1/projects/proj_123")
                .header("Idempotency-Key", "idem_456")
                .header("X-Request-ID", "trace_123");
            then.status(200).json_body(json!({
                "project_id": "proj_123", "name": "Test",
                "enable_signed_ingest": false
            }));
        });

        let client = test_client(&server);
        let opts = RequestOptions {
            idempotency_key: Some("idem_456".to_string()),
            request_id: Some("trace_123".to_string()),
            timeout: None,
            headers: None,
        };
        client
            .get_project_with_opts("proj_123", Some(&opts))
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn request_options_custom_headers() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/v1/projects/proj_123")
                .header("X-Custom", "value");
            then.status(200).json_body(json!({
                "project_id": "proj_123", "name": "Test",
                "enable_signed_ingest": false
            }));
        });

        let client = test_client(&server);
        let mut extra_headers = reqwest::header::HeaderMap::new();
        extra_headers.insert("X-Custom", "value".parse().unwrap());
        let opts = RequestOptions {
            headers: Some(extra_headers),
            ..Default::default()
        };
        client
            .get_project_with_opts("proj_123", Some(&opts))
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn request_options_timeout() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/v1/projects/proj_123");
            then.status(200)
                .delay(std::time::Duration::from_millis(200))
                .json_body(json!({
                    "project_id": "proj_123", "name": "Test",
                    "enable_signed_ingest": false
                }));
        });

        let client = test_client(&server);
        let opts = RequestOptions {
            timeout: Some(std::time::Duration::from_millis(10)),
            ..Default::default()
        };
        let err = client
            .get_project_with_opts("proj_123", Some(&opts))
            .await
            .unwrap_err();
        assert!(err.is_transport());
    }

    // ── Metadata in requests ───────────────────────────────────────

    #[tokio::test]
    async fn create_breaker_with_metadata_in_body() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/v1/projects/proj_123/breakers");
            then.status(201).json_body(json!({
                "breaker": {
                    "id": "b1", "name": "test",
                    "kind": "error_rate", "metric": "latency", "threshold": 100.0,
                    "op": "gt",
                    "metadata": {"region": "us-east-1"}
                }
            }));
        });

        let client = test_client(&server);
        let input = CreateBreakerInput {
            name: "test".to_string(),
            metric: "latency".to_string(),
            kind: BreakerKind::ErrorRate,
            kind_params: None,
            threshold: 100.0,
            op: BreakerOp::Gt,
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
            metadata: Some(std::collections::HashMap::from([(
                "region".to_string(),
                "us-east-1".to_string(),
            )])),
        };
        let breaker = client.create_breaker("proj_123", &input).await.unwrap();

        mock.assert();
        assert!(breaker.metadata.is_some());
    }

    #[tokio::test]
    async fn create_breaker_nil_metadata_omitted() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/v1/projects/proj_123/breakers");
            then.status(201).json_body(json!({
                "breaker": {
                    "id": "b2", "name": "no-meta",
                    "kind": "error_rate", "metric": "latency", "threshold": 100.0,
                    "op": "gt"
                }
            }));
        });

        let client = test_client(&server);
        let input = CreateBreakerInput {
            name: "no-meta".to_string(),
            metric: "latency".to_string(),
            kind: BreakerKind::ErrorRate,
            kind_params: None,
            threshold: 100.0,
            op: BreakerOp::Gt,
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
            metadata: None,
        };
        let breaker = client.create_breaker("proj_123", &input).await.unwrap();

        mock.assert();
        assert!(breaker.metadata.is_none());
    }

    // ── Pager Tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn pager_iterates_across_pages() {
        let server = MockServer::start();
        // Register the specific cursor mock first so it matches before the general one
        server.mock(|when, then| {
            when.method(GET)
                .path("/v1/projects/proj_123/events")
                .query_param("cursor", "cursor_page2");
            then.status(200).json_body(json!({
                "events": [
                    {"id":"e3","project_id":"proj_123","breaker_id":"b1","from_state":"closed","to_state":"open","timestamp":"2024-01-03T00:00:00Z"}
                ],
                "returned": 1
            }));
        });
        server.mock(|when, then| {
            when.method(GET)
                .path("/v1/projects/proj_123/events");
            then.status(200).json_body(json!({
                "events": [
                    {"id":"e1","project_id":"proj_123","breaker_id":"b1","from_state":"closed","to_state":"open","timestamp":"2024-01-01T00:00:00Z"},
                    {"id":"e2","project_id":"proj_123","breaker_id":"b1","from_state":"open","to_state":"closed","timestamp":"2024-01-02T00:00:00Z"}
                ],
                "returned": 2,
                "next_cursor": "cursor_page2"
            }));
        });

        let client = test_client(&server);
        let mut pager = client.list_events_pager("proj_123", None);
        let all = pager.collect_all().await.unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].id, "e1");
        assert_eq!(all[2].id, "e3");
    }

    #[tokio::test]
    async fn pager_empty_result() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/v1/projects/proj_123/events");
            then.status(200).json_body(json!({
                "events": [],
                "returned": 0
            }));
        });

        let client = test_client(&server);
        let mut pager = client.list_events_pager("proj_123", None);
        let result = pager.next().await.unwrap();
        assert!(result.is_none());
    }
}
