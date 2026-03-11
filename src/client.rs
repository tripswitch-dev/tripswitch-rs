use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use rand::Rng;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::errors::{Error, ExecuteError};
use crate::ingest::{self, IngestHandle};
use crate::metadata::{self, MetadataHandle};
use crate::sse::{self, SseHandle};
use crate::types::*;

const DEFAULT_BASE_URL: &str = "https://api.tripswitch.dev";

// ── ExecuteOptions ─────────────────────────────────────────────────

type BreakerSelector = Box<dyn FnOnce(&[BreakerMeta]) -> Vec<String> + Send>;
type RouterSelector = Box<dyn FnOnce(&[RouterMeta]) -> String + Send>;
type ErrorEvaluator = Box<dyn FnOnce(&dyn std::error::Error) -> bool + Send>;

/// Options for a single `execute()` call.
pub struct ExecuteOptions {
    pub(crate) breakers: Option<Vec<String>>,
    pub(crate) select_breakers: Option<BreakerSelector>,
    pub(crate) router: Option<String>,
    pub(crate) select_router: Option<RouterSelector>,
    pub(crate) metrics: Option<HashMap<String, MetricValue>>,
    pub(crate) tags: Option<HashMap<String, String>>,
    pub(crate) error_evaluator: Option<ErrorEvaluator>,
    pub(crate) trace_id: Option<String>,
}

impl ExecuteOptions {
    pub fn new() -> Self {
        Self {
            breakers: None,
            select_breakers: None,
            router: None,
            select_router: None,
            metrics: None,
            tags: None,
            error_evaluator: None,
            trace_id: None,
        }
    }

    pub fn breakers(mut self, names: &[&str]) -> Self {
        self.breakers = Some(names.iter().map(|s| s.to_string()).collect());
        self
    }

    pub fn select_breakers<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&[BreakerMeta]) -> Vec<String> + Send + 'static,
    {
        self.select_breakers = Some(Box::new(f));
        self
    }

    pub fn router(mut self, router_id: impl Into<String>) -> Self {
        self.router = Some(router_id.into());
        self
    }

    pub fn select_router<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&[RouterMeta]) -> String + Send + 'static,
    {
        self.select_router = Some(Box::new(f));
        self
    }

    pub fn metrics(mut self, metrics: HashMap<String, MetricValue>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    pub fn metric(mut self, name: impl Into<String>, value: MetricValue) -> Self {
        self.metrics
            .get_or_insert_with(HashMap::new)
            .insert(name.into(), value);
        self
    }

    pub fn tags(mut self, tags: HashMap<String, String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    pub fn trace_id(mut self, id: impl Into<String>) -> Self {
        self.trace_id = Some(id.into());
        self
    }

    pub fn error_evaluator<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&dyn std::error::Error) -> bool + Send + 'static,
    {
        self.error_evaluator = Some(Box::new(f));
        self
    }
}

impl Default for ExecuteOptions {
    fn default() -> Self {
        Self::new()
    }
}

// ── ClientBuilder ──────────────────────────────────────────────────

/// Builder for constructing a [`Client`].
pub struct ClientBuilder {
    project_id: String,
    api_key: Option<String>,
    ingest_secret: Option<String>,
    fail_open: bool,
    base_url: String,
    on_state_change: Option<crate::sse::StateChangeCallback>,
    global_tags: Option<HashMap<String, String>>,
    metadata_sync_interval: Option<Duration>,
    init_timeout: Option<Duration>,
    metadata_sync_disabled: bool,
}

impl ClientBuilder {
    pub fn new(project_id: impl Into<String>) -> Self {
        Self {
            project_id: project_id.into(),
            api_key: None,
            ingest_secret: None,
            fail_open: true,
            base_url: DEFAULT_BASE_URL.to_string(),
            on_state_change: None,
            global_tags: None,
            metadata_sync_interval: None,
            init_timeout: None,
            metadata_sync_disabled: false,
        }
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn ingest_secret(mut self, secret: impl Into<String>) -> Self {
        self.ingest_secret = Some(secret.into());
        self
    }

    pub fn fail_open(mut self, fail_open: bool) -> Self {
        self.fail_open = fail_open;
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn on_state_change<F>(mut self, f: F) -> Self
    where
        F: Fn(&str, BreakerStateValue, BreakerStateValue) + Send + Sync + 'static,
    {
        self.on_state_change = Some(Arc::new(f));
        self
    }

    pub fn global_tags(mut self, tags: HashMap<String, String>) -> Self {
        self.global_tags = Some(tags);
        self
    }

    pub fn metadata_sync_interval(mut self, interval: Duration) -> Self {
        self.metadata_sync_interval = Some(interval);
        self
    }

    pub fn metadata_sync_disabled(mut self, disabled: bool) -> Self {
        self.metadata_sync_disabled = disabled;
        self
    }

    pub fn init_timeout(mut self, timeout: Duration) -> Self {
        self.init_timeout = Some(timeout);
        self
    }

    /// Build the client, connecting to SSE and starting background tasks.
    pub async fn build(self) -> Result<Client, Error> {
        let api_key = self
            .api_key
            .ok_or_else(|| Error::ConflictingOptions("api_key is required".into()))?;
        let base_url = self.base_url.trim_end_matches('/').to_string();
        let cancel = CancellationToken::new();

        // Start SSE listener
        let ready = Arc::new(Notify::new());
        let sse_handle = sse::start_sse_listener(
            base_url.clone(),
            self.project_id.clone(),
            api_key.clone(),
            cancel.clone(),
            ready.clone(),
            self.on_state_change,
        );

        // Start ingest flusher
        let ingest_handle = ingest::start_flusher(
            base_url.clone(),
            self.project_id.clone(),
            self.ingest_secret,
            api_key.clone(),
            cancel.clone(),
        );

        // Start metadata sync
        let metadata_handle = if !self.metadata_sync_disabled {
            Some(metadata::start_metadata_sync(
                base_url.clone(),
                self.project_id.clone(),
                api_key.clone(),
                self.metadata_sync_interval,
                cancel.clone(),
            ))
        } else {
            None
        };

        // Wait for SSE ready (with optional timeout)
        if let Some(timeout) = self.init_timeout {
            match tokio::time::timeout(timeout, ready.notified()).await {
                Ok(()) => {}
                Err(_) => {
                    cancel.cancel();
                    return Err(Error::InitTimeout);
                }
            }
        } else {
            ready.notified().await;
        }

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        let inner = Arc::new(ClientInner {
            project_id: self.project_id,
            base_url,
            api_key,
            fail_open: self.fail_open,
            global_tags: self.global_tags,
            http,
            cancel,
            sse: sse_handle,
            ingest: ingest_handle,
            metadata: metadata_handle,
        });

        Ok(Client { inner })
    }
}

// ── Client ─────────────────────────────────────────────────────────

struct ClientInner {
    project_id: String,
    base_url: String,
    api_key: String,
    #[allow(dead_code)] // wired into ClientBuilder but not yet used in execute()
    fail_open: bool,
    global_tags: Option<HashMap<String, String>>,
    http: reqwest::Client,
    cancel: CancellationToken,
    sse: SseHandle,
    ingest: IngestHandle,
    metadata: Option<MetadataHandle>,
}

/// Runtime client for the Tripswitch circuit breaker service.
///
/// The client connects to the SSE state stream, caches breaker states,
/// and provides the `execute()` method for gating task execution.
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

impl Client {
    /// Create a new `ClientBuilder`.
    pub fn builder(project_id: impl Into<String>) -> ClientBuilder {
        ClientBuilder::new(project_id)
    }

    /// Execute a task, gated by breaker state.
    pub async fn execute<T, E, Fut>(
        &self,
        task: impl FnOnce() -> Fut,
        opts: Option<ExecuteOptions>,
    ) -> Result<T, ExecuteError<E>>
    where
        E: std::error::Error + Send + 'static,
        Fut: Future<Output = Result<T, E>>,
    {
        let opts = opts.unwrap_or_default();

        // 1. Validate no conflicting options
        if opts.breakers.is_some() && opts.select_breakers.is_some() {
            return Err(Error::ConflictingOptions(
                "cannot use both breakers and select_breakers".into(),
            )
            .into());
        }
        if opts.router.is_some() && opts.select_router.is_some() {
            return Err(Error::ConflictingOptions(
                "cannot use both router and select_router".into(),
            )
            .into());
        }

        // 2. Resolve breaker names
        let breaker_names: Option<Vec<String>> = if let Some(names) = opts.breakers {
            Some(names)
        } else if let Some(selector) = opts.select_breakers {
            let meta = self
                .get_breakers_metadata()
                .ok_or(Error::MetadataUnavailable)?;
            Some(selector(&meta))
        } else {
            None
        };

        // 3. Resolve router
        let router_id: Option<String> = if let Some(id) = opts.router {
            Some(id)
        } else if let Some(selector) = opts.select_router {
            let meta = self
                .get_routers_metadata()
                .ok_or(Error::MetadataUnavailable)?;
            Some(selector(&meta))
        } else {
            None
        };

        // 4. Check breaker states
        if let Some(ref names) = breaker_names {
            let states = self.inner.sse.states.read().await;
            let mut min_allow_rate: Option<f64> = None;

            for name in names {
                if let Some(entry) = states.get(name) {
                    match entry.state {
                        BreakerStateValue::Open => {
                            return Err(Error::BreakerOpen.into());
                        }
                        BreakerStateValue::HalfOpen => {
                            if let Some(rate) = entry.allow_rate {
                                min_allow_rate = Some(match min_allow_rate {
                                    Some(current) => current.min(rate),
                                    None => rate,
                                });
                            }
                        }
                        BreakerStateValue::Closed => {}
                    }
                }
                // If breaker not found in state map, treat as closed (fail-open)
            }

            // Half-open probabilistic gate
            if let Some(rate) = min_allow_rate {
                if rate < 1.0 {
                    let r: f64 = rand::rng().random();
                    if r > rate {
                        return Err(Error::BreakerOpen.into());
                    }
                }
            }
        }

        // 5. Run the task
        let start = std::time::Instant::now();
        let result = task().await;
        let duration = start.elapsed();
        let duration_ms = duration.as_secs_f64() * 1000.0;

        // 6. Determine success/failure
        let ok = match &result {
            Ok(_) => true,
            Err(e) => {
                if let Some(evaluator) = opts.error_evaluator {
                    !evaluator(e)
                } else {
                    false
                }
            }
        };

        // 7. Resolve metrics and emit samples
        if let Some(ref rid) = router_id {
            if let Some(metrics) = opts.metrics {
                let mut merged_tags = self.inner.global_tags.clone().unwrap_or_default();
                if let Some(call_tags) = opts.tags {
                    merged_tags.extend(call_tags);
                }

                let ts_ms = chrono::Utc::now().timestamp_millis();

                for (metric_name, metric_val) in metrics {
                    if metric_name.is_empty() {
                        continue;
                    }
                    let value = match metric_val {
                        MetricValue::Latency => duration_ms,
                        MetricValue::Static(v) => v,
                        MetricValue::Dynamic(f) => {
                            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
                                Ok(v) => v,
                                Err(_) => {
                                    warn!("metric closure panicked for '{metric_name}'");
                                    continue;
                                }
                            }
                        }
                    };

                    let entry = ReportEntry {
                        router_id: rid.clone(),
                        metric: metric_name,
                        ts_ms,
                        value,
                        ok,
                        tags: if merged_tags.is_empty() {
                            None
                        } else {
                            Some(merged_tags.clone())
                        },
                        trace_id: opts.trace_id.clone(),
                    };

                    if self.inner.ingest.tx.try_send(entry).is_err() {
                        self.inner
                            .ingest
                            .stats
                            .dropped_samples
                            .fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }

        result.map_err(ExecuteError::Task)
    }

    /// Report a sample directly without executing a task.
    pub fn report(&self, input: ReportInput) {
        let mut merged_tags = self.inner.global_tags.clone().unwrap_or_default();
        if let Some(tags) = input.tags {
            merged_tags.extend(tags);
        }

        let ts_ms = chrono::Utc::now().timestamp_millis();

        let value = match input.value {
            MetricValue::Latency => {
                warn!("MetricValue::Latency not valid for report(), use Static or Dynamic");
                return;
            }
            MetricValue::Static(v) => v,
            MetricValue::Dynamic(f) => {
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
                    Ok(v) => v,
                    Err(_) => {
                        warn!("metric closure panicked in report()");
                        return;
                    }
                }
            }
        };

        let entry = ReportEntry {
            router_id: input.router_id,
            metric: input.metric,
            ts_ms,
            value,
            ok: input.ok,
            tags: if merged_tags.is_empty() {
                None
            } else {
                Some(merged_tags)
            },
            trace_id: input.trace_id,
        };

        if self.inner.ingest.tx.try_send(entry).is_err() {
            self.inner
                .ingest
                .stats
                .dropped_samples
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get the current state of a specific breaker by name.
    pub async fn get_state(&self, breaker_name: &str) -> Option<BreakerStatus> {
        let states = self.inner.sse.states.read().await;
        states.get(breaker_name).map(|e| BreakerStatus {
            name: e.breaker.clone(),
            state: e.state,
            allow_rate: e.allow_rate,
        })
    }

    /// Get the current state of all breakers.
    pub async fn get_all_states(&self) -> Vec<BreakerStatus> {
        let states = self.inner.sse.states.read().await;
        states
            .values()
            .map(|e| BreakerStatus {
                name: e.breaker.clone(),
                state: e.state,
                allow_rate: e.allow_rate,
            })
            .collect()
    }

    /// Get the project status from the API.
    pub async fn get_status(&self) -> Result<Status, Error> {
        let url = format!(
            "{}/v1/projects/{}/status",
            self.inner.base_url, self.inner.project_id
        );
        let resp = self
            .inner
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.inner.api_key))
            .header("X-Contract-Version", crate::CONTRACT_VERSION)
            .send()
            .await?
            .json::<Status>()
            .await?;
        Ok(resp)
    }

    /// Get cached breaker metadata.
    pub fn get_breakers_metadata(&self) -> Option<Vec<BreakerMeta>> {
        if let Some(ref meta) = self.inner.metadata {
            // Try to read without blocking
            meta.cache.breakers.try_read().ok().and_then(|g| g.clone())
        } else {
            None
        }
    }

    /// Get cached router metadata.
    pub fn get_routers_metadata(&self) -> Option<Vec<RouterMeta>> {
        if let Some(ref meta) = self.inner.metadata {
            meta.cache.routers.try_read().ok().and_then(|g| g.clone())
        } else {
            None
        }
    }

    /// Get SDK statistics.
    pub fn stats(&self) -> SdkStats {
        SdkStats {
            dropped_samples: self
                .inner
                .ingest
                .stats
                .dropped_samples
                .load(Ordering::Relaxed),
            buffer_capacity: self.inner.ingest.buffer_capacity(),
            sse_connected: self.inner.sse.stats.connected.load(Ordering::Relaxed),
            sse_reconnects: self.inner.sse.stats.reconnects.load(Ordering::Relaxed),
        }
    }

    /// Gracefully shut down the client.
    pub async fn close(self) {
        debug!("closing tripswitch client");
        self.inner.cancel.cancel();
        match Arc::try_unwrap(self.inner) {
            Ok(inner) => {
                let _ = inner.sse.task_handle.await;
                let _ = inner.ingest.task_handle.await;
                if let Some(meta) = inner.metadata {
                    let _ = meta.task_handle.await;
                }
            }
            Err(_arc) => {
                // Other clones still exist; tasks will stop via cancellation token.
                // Best-effort sleep for drain.
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{IngestHandle, IngestStats};
    use crate::sse::{SseHandle, SseStats};
    use crate::types::{BreakerStateEntry, BreakerStateValue, ReportEntry};
    use std::sync::atomic::{AtomicBool, AtomicU64};
    use tokio::sync::{mpsc, RwLock};

    /// Create a test Client with controllable breaker states and a receivable
    /// ingest channel. No real SSE, flusher, or metadata sync tasks.
    fn new_test_client() -> (Client, mpsc::Receiver<ReportEntry>) {
        new_test_client_with_opts(true, None)
    }

    fn new_test_client_with_opts(
        fail_open: bool,
        global_tags: Option<HashMap<String, String>>,
    ) -> (Client, mpsc::Receiver<ReportEntry>) {
        let (tx, rx) = mpsc::channel::<ReportEntry>(100);

        let states = Arc::new(RwLock::new(HashMap::new()));
        let sse = SseHandle {
            states,
            stats: SseStats {
                connected: Arc::new(AtomicBool::new(true)),
                reconnects: Arc::new(AtomicU64::new(0)),
            },
            task_handle: tokio::spawn(async {}),
        };

        let ingest = IngestHandle {
            tx,
            stats: IngestStats {
                dropped_samples: Arc::new(AtomicU64::new(0)),
            },
            task_handle: tokio::spawn(async {}),
        };

        let inner = Arc::new(ClientInner {
            project_id: "proj_test".to_string(),
            base_url: "https://test.api.dev".to_string(),
            api_key: "eb_pk_test".to_string(),
            fail_open,
            global_tags,
            http: reqwest::Client::new(),
            cancel: CancellationToken::new(),
            sse,
            ingest,
            metadata: None,
        });

        (Client { inner }, rx)
    }

    /// Insert a breaker state into the client's SSE state cache.
    async fn set_breaker_state(
        client: &Client,
        name: &str,
        state: BreakerStateValue,
        allow_rate: Option<f64>,
    ) {
        let mut states = client.inner.sse.states.write().await;
        states.insert(
            name.to_string(),
            BreakerStateEntry {
                breaker: name.to_string(),
                state,
                allow_rate,
            },
        );
    }

    // ── ExecuteOptions Builder ──────────────────────────────────────

    #[test]
    fn execute_options_default_all_none() {
        let opts = ExecuteOptions::new();
        assert!(opts.breakers.is_none());
        assert!(opts.select_breakers.is_none());
        assert!(opts.router.is_none());
        assert!(opts.select_router.is_none());
        assert!(opts.metrics.is_none());
        assert!(opts.tags.is_none());
        assert!(opts.trace_id.is_none());
    }

    #[test]
    fn execute_options_breakers() {
        let opts = ExecuteOptions::new().breakers(&["b1", "b2"]);
        let names = opts.breakers.unwrap();
        assert_eq!(names, vec!["b1".to_string(), "b2".to_string()]);
    }

    #[test]
    fn execute_options_router() {
        let opts = ExecuteOptions::new().router("r1");
        assert_eq!(opts.router.unwrap(), "r1");
    }

    #[test]
    fn execute_options_metric_accumulates() {
        let opts = ExecuteOptions::new()
            .metric("latency", MetricValue::Latency)
            .metric("count", MetricValue::Static(1.0));
        let metrics = opts.metrics.unwrap();
        assert_eq!(metrics.len(), 2);
        assert!(metrics.contains_key("latency"));
        assert!(metrics.contains_key("count"));
    }

    #[test]
    fn execute_options_tag_accumulates() {
        let opts = ExecuteOptions::new()
            .tag("env", "prod")
            .tag("region", "us-east-1");
        let tags = opts.tags.unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags.get("env").unwrap(), "prod");
        assert_eq!(tags.get("region").unwrap(), "us-east-1");
    }

    #[test]
    fn execute_options_trace_id() {
        let opts = ExecuteOptions::new().trace_id("t-123");
        assert_eq!(opts.trace_id.unwrap(), "t-123");
    }

    #[test]
    fn execute_options_chaining() {
        let opts = ExecuteOptions::new()
            .breakers(&["b1"])
            .router("r1")
            .metric("latency", MetricValue::Latency)
            .tag("env", "prod")
            .trace_id("t-1");
        assert!(opts.breakers.is_some());
        assert!(opts.router.is_some());
        assert!(opts.metrics.is_some());
        assert!(opts.tags.is_some());
        assert!(opts.trace_id.is_some());
    }

    // ── ClientBuilder ──────────────────────────────────────────────

    #[test]
    fn client_builder_defaults() {
        let builder = ClientBuilder::new("proj_123");
        assert_eq!(builder.project_id, "proj_123");
        assert!(builder.api_key.is_none());
        assert!(builder.ingest_secret.is_none());
        assert!(builder.fail_open);
        assert_eq!(builder.base_url, DEFAULT_BASE_URL);
        assert!(builder.global_tags.is_none());
        assert!(builder.metadata_sync_interval.is_none());
        assert!(!builder.metadata_sync_disabled);
        assert!(builder.init_timeout.is_none());
    }

    #[test]
    fn client_builder_setters() {
        let tags = HashMap::from([("env".to_string(), "prod".to_string())]);
        let builder = ClientBuilder::new("proj_123")
            .api_key("eb_pk_test")
            .ingest_secret("secret123")
            .fail_open(false)
            .base_url("https://custom.api.dev")
            .global_tags(tags.clone())
            .metadata_sync_interval(Duration::from_secs(60))
            .metadata_sync_disabled(true)
            .init_timeout(Duration::from_secs(5));

        assert_eq!(builder.api_key.unwrap(), "eb_pk_test");
        assert_eq!(builder.ingest_secret.unwrap(), "secret123");
        assert!(!builder.fail_open);
        assert_eq!(builder.base_url, "https://custom.api.dev");
        assert_eq!(builder.global_tags.unwrap(), tags);
        assert_eq!(
            builder.metadata_sync_interval.unwrap(),
            Duration::from_secs(60)
        );
        assert!(builder.metadata_sync_disabled);
        assert_eq!(builder.init_timeout.unwrap(), Duration::from_secs(5));
    }

    // ── Execute flow tests ─────────────────────────────────────────

    #[tokio::test]
    async fn execute_no_breakers_passthrough() {
        let (client, _rx) = new_test_client();
        let result: Result<String, std::io::Error> = client
            .execute(|| async { Ok("success".to_string()) }, None)
            .await
            .map_err(|e| match e {
                ExecuteError::Task(e) => e,
                ExecuteError::Sdk(e) => panic!("unexpected sdk error: {e}"),
            });
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn execute_closed_breaker_allows() {
        let (client, _rx) = new_test_client();
        set_breaker_state(
            &client,
            "test-breaker",
            BreakerStateValue::Closed,
            Some(1.0),
        )
        .await;

        let result = client
            .execute(
                || async { Ok::<_, std::io::Error>("success".to_string()) },
                Some(ExecuteOptions::new().breakers(&["test-breaker"])),
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn execute_open_breaker_blocks() {
        let (client, _rx) = new_test_client();
        set_breaker_state(&client, "test-breaker", BreakerStateValue::Open, Some(0.0)).await;

        let mut task_ran = false;
        let result = client
            .execute(
                || async {
                    task_ran = true;
                    Ok::<_, std::io::Error>("should not run".to_string())
                },
                Some(ExecuteOptions::new().breakers(&["test-breaker"])),
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_breaker_error());
        assert!(!task_ran);
    }

    #[tokio::test]
    async fn execute_half_open_allow_rate_zero_blocks() {
        let (client, _rx) = new_test_client();
        set_breaker_state(&client, "hb", BreakerStateValue::HalfOpen, Some(0.0)).await;

        let result = client
            .execute(
                || async { Ok::<_, std::io::Error>("nope".to_string()) },
                Some(ExecuteOptions::new().breakers(&["hb"])),
            )
            .await;
        assert!(result.unwrap_err().is_breaker_error());
    }

    #[tokio::test]
    async fn execute_half_open_allow_rate_one_allows() {
        let (client, _rx) = new_test_client();
        set_breaker_state(&client, "hb", BreakerStateValue::HalfOpen, Some(1.0)).await;

        let result = client
            .execute(
                || async { Ok::<_, std::io::Error>("allowed".to_string()) },
                Some(ExecuteOptions::new().breakers(&["hb"])),
            )
            .await;
        assert_eq!(result.unwrap(), "allowed");
    }

    #[tokio::test]
    async fn execute_unknown_breaker_fail_open() {
        let (client, _rx) = new_test_client();
        // Don't set any breaker state — unknown breaker should fail-open

        let result = client
            .execute(
                || async { Ok::<_, std::io::Error>("success".to_string()) },
                Some(ExecuteOptions::new().breakers(&["nonexistent"])),
            )
            .await;
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn execute_multiple_breakers_any_open_blocks() {
        let (client, _rx) = new_test_client();
        set_breaker_state(&client, "a", BreakerStateValue::Closed, Some(1.0)).await;
        set_breaker_state(&client, "b", BreakerStateValue::Open, Some(0.0)).await;

        let result = client
            .execute(
                || async { Ok::<_, std::io::Error>("nope".to_string()) },
                Some(ExecuteOptions::new().breakers(&["a", "b"])),
            )
            .await;
        assert!(result.unwrap_err().is_breaker_error());
    }

    #[tokio::test]
    async fn execute_multiple_breakers_all_closed_allows() {
        let (client, _rx) = new_test_client();
        set_breaker_state(&client, "a", BreakerStateValue::Closed, Some(1.0)).await;
        set_breaker_state(&client, "b", BreakerStateValue::Closed, Some(1.0)).await;

        let result = client
            .execute(
                || async { Ok::<_, std::io::Error>("success".to_string()) },
                Some(ExecuteOptions::new().breakers(&["a", "b"])),
            )
            .await;
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn execute_multiple_half_open_uses_min_allow_rate() {
        let (client, _rx) = new_test_client();
        set_breaker_state(&client, "a", BreakerStateValue::HalfOpen, Some(0.2)).await;
        set_breaker_state(&client, "b", BreakerStateValue::HalfOpen, Some(0.5)).await;

        let mut allowed = 0u32;
        let iterations = 10_000u32;
        for _ in 0..iterations {
            let result = client
                .execute(
                    || async { Ok::<_, std::io::Error>("ok".to_string()) },
                    Some(ExecuteOptions::new().breakers(&["a", "b"])),
                )
                .await;
            if result.is_ok() {
                allowed += 1;
            }
        }
        let rate = allowed as f64 / iterations as f64;
        // Should be ~0.2 (min), not ~0.1 (multiplicative)
        assert!(rate > 0.17 && rate < 0.23, "expected rate ~0.2, got {rate}");
    }

    // ── Conflicting options ────────────────────────────────────────

    #[tokio::test]
    async fn execute_conflicting_breakers_and_select_breakers() {
        let (client, _rx) = new_test_client();
        let opts = ExecuteOptions {
            breakers: Some(vec!["b1".to_string()]),
            select_breakers: Some(Box::new(|_| vec!["b2".to_string()])),
            ..ExecuteOptions::new()
        };
        let result = client
            .execute(
                || async { Ok::<_, std::io::Error>("nope".to_string()) },
                Some(opts),
            )
            .await;
        let err = result.unwrap_err();
        match err.sdk_error().unwrap() {
            Error::ConflictingOptions(msg) => {
                assert!(msg.contains("breakers"));
            }
            other => panic!("expected ConflictingOptions, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn execute_conflicting_router_and_select_router() {
        let (client, _rx) = new_test_client();
        let opts = ExecuteOptions {
            router: Some("r1".to_string()),
            select_router: Some(Box::new(|_| "r2".to_string())),
            ..ExecuteOptions::new()
        };
        let result = client
            .execute(
                || async { Ok::<_, std::io::Error>("nope".to_string()) },
                Some(opts),
            )
            .await;
        let err = result.unwrap_err();
        assert!(matches!(
            err.sdk_error().unwrap(),
            Error::ConflictingOptions(_)
        ));
    }

    // ── Metrics / Samples ──────────────────────────────────────────

    #[tokio::test]
    async fn execute_with_latency_metric_emits_sample() {
        let (client, mut rx) = new_test_client();

        let _result = client
            .execute(
                || async {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    Ok::<_, std::io::Error>("ok".to_string())
                },
                Some(
                    ExecuteOptions::new()
                        .router("r1")
                        .metric("latency", MetricValue::Latency),
                ),
            )
            .await
            .unwrap();

        let sample = rx.try_recv().unwrap();
        assert_eq!(sample.router_id, "r1");
        assert_eq!(sample.metric, "latency");
        assert!(sample.value >= 0.0);
        assert!(sample.ok);
    }

    #[tokio::test]
    async fn execute_no_metrics_no_samples() {
        let (client, mut rx) = new_test_client();

        let _result = client
            .execute(
                || async { Ok::<_, std::io::Error>("ok".to_string()) },
                Some(ExecuteOptions::new().router("r1")),
            )
            .await
            .unwrap();

        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn execute_no_router_no_samples_even_with_metrics() {
        let (client, mut rx) = new_test_client();

        let _result = client
            .execute(
                || async { Ok::<_, std::io::Error>("ok".to_string()) },
                Some(ExecuteOptions::new().metric("latency", MetricValue::Latency)),
            )
            .await
            .unwrap();

        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn execute_tags_merge_global_and_call() {
        let global = HashMap::from([
            ("env".to_string(), "test".to_string()),
            ("service".to_string(), "api".to_string()),
        ]);
        let (client, mut rx) = new_test_client_with_opts(true, Some(global));

        let _result = client
            .execute(
                || async { Ok::<_, std::io::Error>("ok".to_string()) },
                Some(
                    ExecuteOptions::new()
                        .router("r1")
                        .metric("latency", MetricValue::Latency)
                        .tag("env", "override")
                        .tag("endpoint", "/users"),
                ),
            )
            .await
            .unwrap();

        let sample = rx.try_recv().unwrap();
        let tags = sample.tags.unwrap();
        // Call tag overrides global
        assert_eq!(tags.get("env").unwrap(), "override");
        // Global tag preserved
        assert_eq!(tags.get("service").unwrap(), "api");
        // Call-specific tag
        assert_eq!(tags.get("endpoint").unwrap(), "/users");
    }

    #[tokio::test]
    async fn execute_trace_id_in_sample() {
        let (client, mut rx) = new_test_client();

        let _result = client
            .execute(
                || async { Ok::<_, std::io::Error>("ok".to_string()) },
                Some(
                    ExecuteOptions::new()
                        .router("r1")
                        .metric("latency", MetricValue::Latency)
                        .trace_id("trace-abc-123"),
                ),
            )
            .await
            .unwrap();

        let sample = rx.try_recv().unwrap();
        assert_eq!(sample.trace_id.as_deref(), Some("trace-abc-123"));
    }

    #[tokio::test]
    async fn execute_with_multiple_metrics() {
        let (client, mut rx) = new_test_client();

        let _result = client
            .execute(
                || async { Ok::<_, std::io::Error>("ok".to_string()) },
                Some(
                    ExecuteOptions::new()
                        .router("r1")
                        .metric("latency", MetricValue::Latency)
                        .metric("count", MetricValue::Static(1.0))
                        .metric("amount", MetricValue::Static(99.99)),
                ),
            )
            .await
            .unwrap();

        let mut samples = Vec::new();
        while let Ok(s) = rx.try_recv() {
            samples.push(s);
        }
        assert_eq!(samples.len(), 3);

        let metrics: Vec<&str> = samples.iter().map(|s| s.metric.as_str()).collect();
        assert!(metrics.contains(&"latency"));
        assert!(metrics.contains(&"count"));
        assert!(metrics.contains(&"amount"));

        let count_sample = samples.iter().find(|s| s.metric == "count").unwrap();
        assert_eq!(count_sample.value, 1.0);

        let amount_sample = samples.iter().find(|s| s.metric == "amount").unwrap();
        assert_eq!(amount_sample.value, 99.99);
    }

    #[tokio::test]
    async fn execute_with_dynamic_metric() {
        let (client, mut rx) = new_test_client();

        let _result = client
            .execute(
                || async { Ok::<_, std::io::Error>("ok".to_string()) },
                Some(
                    ExecuteOptions::new()
                        .router("r1")
                        .metric("queue_depth", MetricValue::Dynamic(Box::new(|| 42.0))),
                ),
            )
            .await
            .unwrap();

        let sample = rx.try_recv().unwrap();
        assert_eq!(sample.metric, "queue_depth");
        assert_eq!(sample.value, 42.0);
    }

    #[tokio::test]
    async fn execute_router_id_in_samples() {
        let (client, mut rx) = new_test_client();

        let _result = client
            .execute(
                || async { Ok::<_, std::io::Error>("ok".to_string()) },
                Some(
                    ExecuteOptions::new()
                        .router("custom-router-123")
                        .metric("latency", MetricValue::Latency),
                ),
            )
            .await
            .unwrap();

        let sample = rx.try_recv().unwrap();
        assert_eq!(sample.router_id, "custom-router-123");
    }

    #[tokio::test]
    async fn execute_task_error_sample_ok_false() {
        let (client, mut rx) = new_test_client();

        let result = client
            .execute(
                || async { Err::<String, _>(std::io::Error::other("task failed")) },
                Some(
                    ExecuteOptions::new()
                        .router("r1")
                        .metric("latency", MetricValue::Latency),
                ),
            )
            .await;

        assert!(result.is_err());
        let sample = rx.try_recv().unwrap();
        assert!(!sample.ok);
    }

    #[tokio::test]
    async fn execute_gating_only_no_metrics() {
        let (client, mut rx) = new_test_client();
        set_breaker_state(&client, "b1", BreakerStateValue::Closed, Some(1.0)).await;

        let result = client
            .execute(
                || async { Ok::<_, std::io::Error>("success".to_string()) },
                Some(ExecuteOptions::new().breakers(&["b1"])),
            )
            .await;

        assert_eq!(result.unwrap(), "success");
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn execute_metrics_only_no_gating() {
        let (client, mut rx) = new_test_client();

        let result = client
            .execute(
                || async { Ok::<_, std::io::Error>("success".to_string()) },
                Some(
                    ExecuteOptions::new()
                        .router("r1")
                        .metric("latency", MetricValue::Latency),
                ),
            )
            .await;

        assert_eq!(result.unwrap(), "success");
        let sample = rx.try_recv().unwrap();
        assert_eq!(sample.router_id, "r1");
    }

    // ── get_state / get_all_states ─────────────────────────────────

    #[tokio::test]
    async fn get_state_unknown_returns_none() {
        let (client, _rx) = new_test_client();
        assert!(client.get_state("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn get_state_known_returns_status() {
        let (client, _rx) = new_test_client();
        set_breaker_state(&client, "b1", BreakerStateValue::Open, Some(0.0)).await;

        let status = client.get_state("b1").await.unwrap();
        assert_eq!(status.name, "b1");
        assert_eq!(status.state, BreakerStateValue::Open);
        assert_eq!(status.allow_rate, Some(0.0));
    }

    #[tokio::test]
    async fn get_all_states_empty() {
        let (client, _rx) = new_test_client();
        let states = client.get_all_states().await;
        assert!(states.is_empty());
    }

    #[tokio::test]
    async fn get_all_states_returns_all() {
        let (client, _rx) = new_test_client();
        set_breaker_state(&client, "a", BreakerStateValue::Closed, Some(1.0)).await;
        set_breaker_state(&client, "b", BreakerStateValue::Open, Some(0.0)).await;
        set_breaker_state(&client, "c", BreakerStateValue::HalfOpen, Some(0.5)).await;

        let states = client.get_all_states().await;
        assert_eq!(states.len(), 3);
    }

    // ── stats ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn stats_reflects_counters() {
        let (client, _rx) = new_test_client();
        client
            .inner
            .ingest
            .stats
            .dropped_samples
            .store(5, Ordering::Relaxed);
        client
            .inner
            .sse
            .stats
            .connected
            .store(true, Ordering::Relaxed);

        let stats = client.stats();
        assert_eq!(stats.dropped_samples, 5);
        assert!(stats.sse_connected);
    }
}
