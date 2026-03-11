# tripswitch-rs

[![Crates.io](https://img.shields.io/crates/v/tripswitch.svg)](https://crates.io/crates/tripswitch)
[![docs.rs](https://docs.rs/tripswitch/badge.svg)](https://docs.rs/tripswitch)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Official Rust client SDK for [Tripswitch](https://tripswitch.dev) — a circuit breaker management service.

This SDK conforms to the [Tripswitch SDK Contract v0.2](https://tripswitch.dev/docs/sdk-contract).

## Features

- **Real-time state sync** via Server-Sent Events (SSE)
- **Automatic sample reporting** with buffered, gzip-compressed, batched uploads
- **Fail-open by default** — your app stays available even if Tripswitch is unreachable
- **Thread-safe** — one client per project, `Clone + Send + Sync`
- **Graceful shutdown** with cancellation token and sample flushing

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
tripswitch = "0.1"
tokio = { version = "1", features = ["rt", "macros"] }
```

**Requires Rust 1.75+** (uses `async fn` in traits)

## Authentication

Tripswitch uses a two-tier authentication model:

### Runtime Credentials (SDK)

For SDK initialization, you need two credentials from **Project Settings → SDK Keys**:

| Credential | Prefix | Purpose |
|------------|--------|---------|
| **Project Key** | `eb_pk_` | SSE connection and state reads |
| **Ingest Secret** | `ik_` | HMAC-signed sample ingestion |

```rust
let client = Client::builder("proj_abc123")
    .api_key("eb_pk_...")       // Project key
    .ingest_secret("ik_...")    // Ingest secret
    .build()
    .await?;
```

### Admin Credentials (Management API)

For management and automation tasks, use an **Admin Key** from **Organization Settings → Admin Keys**:

| Credential | Prefix | Purpose |
|------------|--------|---------|
| **Admin Key** | `eb_admin_` | Organization-scoped management operations |

Admin keys are used with the [Admin Client](#admin-client) for creating projects, managing breakers, and other administrative tasks — not for runtime SDK usage.

## Quick Start

```rust
use tripswitch::{Client, ExecuteOptions, MetricValue};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client (blocks until SSE state sync completes)
    let client = Client::builder("proj_abc123")
        .api_key("eb_pk_...")
        .ingest_secret("ik_...")
        .init_timeout(Duration::from_secs(10))
        .build()
        .await?;

    // Wrap operations with circuit breaker
    let body = client
        .execute(
            || async {
                let resp = reqwest::get("https://api.example.com/data").await?;
                resp.text().await
            },
            ExecuteOptions::new()
                .breakers(&["external-api"])
                .router("my-router-id")
                .metric("latency", MetricValue::Latency),
        )
        .await;

    match body {
        Ok(text) => println!("Response: {text}"),
        Err(e) if e.is_breaker_error() => {
            // Circuit is open — return cached/fallback response
            println!("circuit open, using fallback");
        }
        Err(e) => println!("request failed: {e}"),
    }

    client.close().await;
    Ok(())
}
```

## How It Works

1. **State Sync**: The client maintains a local cache of breaker states, updated in real-time via SSE
2. **Execute Check**: Each `execute()` call checks the local cache (no network call)
3. **Sample Reporting**: Results are buffered and batched (500 samples or 15s, whichever comes first), gzip-compressed, and HMAC-signed
4. **Graceful Degradation**: If Tripswitch is unreachable, the client fails open by default

## Circuit Breaker States

| State | Behavior |
|-------|----------|
| `Closed` | All requests allowed, results reported |
| `Open` | All requests rejected with `BreakerOpen` |
| `HalfOpen` | Requests throttled based on `allow_rate` (e.g., 20% allowed) |

## Configuration Options

### Client Options

| Option | Description | Default |
|--------|-------------|---------|
| `.api_key(key)` | Project key (`eb_pk_`) for SSE authentication | Required |
| `.ingest_secret(key)` | Ingest secret (`ik_`) for HMAC-signed sample reporting | Optional |
| `.fail_open(bool)` | Allow traffic when Tripswitch is unreachable | `true` |
| `.base_url(url)` | Override API endpoint | `https://api.tripswitch.dev` |
| `.on_state_change(fn)` | Callback on breaker state transitions | `None` |
| `.global_tags(tags)` | Tags applied to all samples | `None` |
| `.metadata_sync_interval(d)` | Interval for refreshing breaker/router metadata from the API | `30s` |
| `.metadata_sync_disabled(bool)` | Disable metadata sync entirely | `false` |
| `.init_timeout(d)` | Maximum time to wait for initial SSE sync | No timeout |

### Execute Options

| Option | Description |
|--------|-------------|
| `.breakers(&["name"])` | Breaker names to check before executing (any open → `BreakerOpen`). If omitted, no gating is performed. |
| `.select_breakers(fn)` | Dynamically select breakers based on cached metadata. Mutually exclusive with `.breakers()`. |
| `.router("id")` | Router ID for sample routing. If omitted, no samples are emitted. |
| `.select_router(fn)` | Dynamically select a router based on cached metadata. Mutually exclusive with `.router()`. |
| `.metric(name, value)` | Add a metric to report (`MetricValue::Latency`, `Static(f64)`, or `Dynamic(closure)`) |
| `.metrics(map)` | Set all metrics at once |
| `.tag(key, value)` | Add a single diagnostic tag |
| `.tags(map)` | Diagnostic tags for this specific call (merged with global tags) |
| `.error_evaluator(fn)` | Custom function to determine if error is a failure |
| `.trace_id(id)` | Explicit trace ID for this call |

### Error Classification

Every sample includes an `ok` field indicating whether the task succeeded or failed. This is determined by the following evaluation order:

1. **`.error_evaluator(fn)`** — if set, takes precedence. The closure receives the error and returns `true` if the error **is a failure**; `false` if it should be treated as success.

   ```rust
   ExecuteOptions::new().error_evaluator(|err| {
       // Only count 5xx as failures
       !err.to_string().contains("404")
   })
   ```

2. **Default** — any `Err` is a failure; `Ok` is success.

## API Reference

### Client::builder

```rust
let client = Client::builder("proj_abc123")
    .api_key("eb_pk_...")
    .build()
    .await?;
```

Creates a new Tripswitch client. Starts background tasks for SSE state sync, sample flushing, and metadata caching. Blocks until the initial SSE sync completes (when an API key is configured). Returns an error if the init timeout expires before sync completes.

### execute

```rust
async fn execute<T, E, Fut>(
    &self,
    task: impl FnOnce() -> Fut,
    opts: ExecuteOptions,
) -> Result<T, ExecuteError<E>>
where
    E: std::error::Error + Send + 'static,
    Fut: Future<Output = Result<T, E>>,
```

Runs a task end-to-end: checks breaker state, executes the task, and reports samples — all in one call.

- Use `.breakers()` to gate execution on breaker state (omit for pass-through)
- Use `.router()` to specify where samples go (omit for no sample emission)
- Use `.metric()` to specify what values to report

Returns `ExecuteError::Sdk(Error::BreakerOpen)` if any specified breaker is open.

### MetricValue

```rust
pub enum MetricValue {
    Latency,                                  // Auto-compute task duration (ms)
    Static(f64),                              // Fixed numeric value
    Dynamic(Box<dyn FnOnce() -> f64 + Send>), // Deferred computation
}
```

`Latency` is a convenience sentinel that auto-computes task duration in milliseconds. You can report **any metric with any value**:

```rust
ExecuteOptions::new()
    .router("my-router")
    .metric("latency", MetricValue::Latency)
    .metric("response_bytes", MetricValue::Static(4096.0))
    .metric("queue_depth", MetricValue::Dynamic(Box::new(|| {
        get_queue_length() as f64
    })))
```

### close

```rust
async fn close(self)
```

Gracefully shuts down the client. Cancels background tasks and gives them time to drain buffered samples.

### stats

```rust
fn stats(&self) -> SdkStats
```

Returns a snapshot of SDK health metrics:

```rust
pub struct SdkStats {
    pub dropped_samples: u64,   // Samples dropped due to buffer overflow
    pub buffer_capacity: usize,  // Channel buffer capacity
    pub sse_connected: bool,    // SSE connection status
    pub sse_reconnects: u64,    // Count of SSE reconnections
}
```

### Breaker State Inspection

These methods expose the SDK's local breaker cache for debugging, logging, and health checks. For gating traffic on breaker state, use `execute` with `.breakers()` — it handles state checks, throttling, and sample reporting together.

```rust
pub struct BreakerStatus {
    pub name: String,
    pub state: BreakerStateValue,  // Open, Closed, HalfOpen
    pub allow_rate: Option<f64>,   // 0.0 to 1.0
}

async fn get_state(&self, breaker_name: &str) -> Option<BreakerStatus>
async fn get_all_states(&self) -> Vec<BreakerStatus>
```

`get_state` returns the cached state of a single breaker, or `None` if not found. `get_all_states` returns a copy of all cached breaker states.

```rust
// Debug: why is checkout rejecting requests?
if let Some(status) = client.get_state("checkout").await {
    println!("checkout breaker: state={:?} allow_rate={:?}", status.state, status.allow_rate);
}

// Health endpoint: expose all breaker states to monitoring
for status in client.get_all_states().await {
    println!("breaker {}: {:?}", status.name, status.state);
}
```

### Error Handling

```rust
pub enum Error {
    BreakerOpen,
    ConflictingOptions(String),
    MetadataUnavailable,
    InitTimeout,
    Api(ApiError),
    Transport(reqwest::Error),
}

pub enum ExecuteError<E> {
    Sdk(Error),   // SDK error (breaker open, conflicting options, etc.)
    Task(E),      // User's task error (preserves original type)
}
```

| Error | Cause |
|-------|-------|
| `BreakerOpen` | A specified breaker is open or request was throttled in half-open state |
| `ConflictingOptions` | Mutually exclusive options used (e.g. `.breakers()` + `.select_breakers()`) |
| `MetadataUnavailable` | Selector used but metadata cache hasn't been populated yet |
| `InitTimeout` | Client initialization timed out waiting for SSE sync |

`ExecuteError<E>` preserves your task's error type — use `is_breaker_error()`, `sdk_error()`, or `task_error()` to inspect:

```rust
let result = client.execute(|| async { do_work().await }, ExecuteOptions::default()).await;
match result {
    Ok(value) => { /* success */ }
    Err(e) if e.is_breaker_error() => {
        // Circuit is open — use fallback
    }
    Err(ExecuteError::Task(e)) => {
        // Your task returned an error
    }
    Err(ExecuteError::Sdk(e)) => {
        // Other SDK error
    }
}
```

## Dynamic Selection

Use `.select_breakers()` and `.select_router()` to choose breakers or routers at runtime based on cached metadata. The SDK periodically syncs metadata from the API (default 30s), and your selector receives the current snapshot.

```rust
use tripswitch::BreakerMeta;

fn breakers_in_region(breakers: &[BreakerMeta]) -> Vec<String> {
    breakers
        .iter()
        .filter(|b| {
            b.metadata.as_ref().and_then(|m| m.get("region"))
                == Some(&"us-east-1".to_string())
        })
        .map(|b| b.name.clone())
        .collect()
}

// Gate on breakers matching a metadata property
let result = client
    .execute(
        || async { do_work().await },
        ExecuteOptions::new().select_breakers(breakers_in_region),
    )
    .await;
```

```rust
use tripswitch::RouterMeta;

fn production_router(routers: &[RouterMeta]) -> String {
    routers
        .iter()
        .find(|r| {
            r.metadata.as_ref().and_then(|m| m.get("env"))
                == Some(&"production".to_string())
        })
        .map(|r| r.id.clone())
        .unwrap_or_default()
}

// Route samples to a router matching a metadata property
let result = client
    .execute(
        || async { do_work().await },
        ExecuteOptions::new()
            .select_router(production_router)
            .metric("latency", MetricValue::Latency),
    )
    .await;
```

**Constraints:**
- `.breakers()` and `.select_breakers()` are mutually exclusive — using both returns `ConflictingOptions`
- `.router()` and `.select_router()` are mutually exclusive — using both returns `ConflictingOptions`
- If the metadata cache hasn't been populated yet, returns `MetadataUnavailable`
- If the selector returns an empty list/string, no gating or sample emission occurs

You can also access the metadata cache directly:

```rust
let breakers: Option<Vec<BreakerMeta>> = client.get_breakers_metadata();
let routers: Option<Vec<RouterMeta>> = client.get_routers_metadata();
```

## Report

```rust
fn report(&self, input: ReportInput)
```

Send a sample independently of `execute`. Use this for async workflows, result-derived metrics, or fire-and-forget reporting:

```rust
use tripswitch::{ReportInput, MetricValue};

// Report token usage from an LLM API response
client.report(ReportInput {
    router_id: "llm-router".to_string(),
    metric: "total_tokens".to_string(),
    value: MetricValue::Static(resp.usage.total_tokens as f64),
    ok: true,
    trace_id: None,
    tags: None,
});

// Background process metrics
client.report(ReportInput {
    router_id: "worker-metrics".to_string(),
    metric: "queue_depth".to_string(),
    value: MetricValue::Static(queue_len as f64),
    ok: true,
    trace_id: None,
    tags: Some(HashMap::from([("worker".into(), "processor-1".into())])),
});
```

Samples are buffered and batched the same way as `execute` samples. Global tags are merged automatically.

## Admin Client

The `admin` module provides a client for management and automation tasks. This is separate from the runtime SDK and uses organization-scoped admin keys.

```rust
use tripswitch::admin::{AdminClient, types::*};

let client = AdminClient::builder("eb_admin_...")
    .build();

// List all projects
let projects = client.list_projects(None).await?;

// Create a project
let project = client
    .create_project(&CreateProjectInput {
        name: "prod-payments".to_string(),
        description: None,
    })
    .await?;

// Get project details
let project = client.get_project("proj_abc123").await?;

// Delete a project (requires name confirmation)
client.delete_project("proj_abc123", "prod-payments").await?;

// List breakers
let params = ListParams { page: Some(1), per_page: Some(100) };
let page = client.list_breakers("proj_abc123", Some(&params)).await?;

// Create a breaker
let breaker = client
    .create_breaker(
        "proj_abc123",
        &CreateBreakerInput {
            name: "api-latency".to_string(),
            metric: "latency_ms".to_string(),
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
        },
    )
    .await?;
```

### Request Options

Every method has a `_with_opts` variant that accepts per-request options:

```rust
use tripswitch::admin::RequestOptions;

let opts = RequestOptions {
    idempotency_key: Some("idem-123".to_string()),
    request_id: Some("trace-456".to_string()),
    timeout: Some(Duration::from_secs(5)),
    headers: None,
};

let project = client.get_project_with_opts("proj_abc123", Some(&opts)).await?;
```

### Admin Error Handling

```rust
use tripswitch::admin::errors::AdminError;

match client.get_project("proj_missing").await {
    Ok(project) => { /* success */ }
    Err(e) if e.is_not_found() => println!("project not found"),
    Err(e) if e.is_rate_limited() => {
        let retry_after = e.api_error().and_then(|a| a.retry_after);
        println!("rate limited, retry after {retry_after:?}s");
    }
    Err(e) if e.is_unauthorized() => println!("invalid API key"),
    Err(e) => println!("error: {e}"),
}
```

| Variant | HTTP Status |
|---------|-------------|
| `NotFound` | 404 |
| `Unauthorized` | 401 |
| `Forbidden` | 403 |
| `RateLimited` | 429 |
| `Conflict` | 409 |
| `Validation` | 422 |
| `ServerFault` | 500–599 |
| `Transport` | Connection error |

**Note:** Admin keys (`eb_admin_`) are for management operations only. For runtime SDK usage, use project keys (`eb_pk_`) as shown in [Quick Start](#quick-start).

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

[Apache License 2.0](LICENSE)
