use std::time::Duration;
use tripswitch::{Client, ExecuteOptions, MetricValue};

fn api_key() -> String {
    std::env::var("TRIPSWITCH_API_KEY").expect("TRIPSWITCH_API_KEY must be set")
}

fn project_id() -> String {
    std::env::var("TRIPSWITCH_PROJECT_ID").expect("TRIPSWITCH_PROJECT_ID must be set")
}

async fn make_client() -> Client {
    let mut builder = Client::builder(project_id())
        .api_key(api_key())
        .init_timeout(Duration::from_secs(10));
    if let Ok(url) = std::env::var("TRIPSWITCH_BASE_URL") {
        builder = builder.base_url(url);
    }
    builder.build().await.expect("failed to build client")
}

#[tokio::test]
#[ignore]
async fn test_new_client() {
    let client = make_client().await;
    let stats = client.stats();
    assert!(stats.sse_connected);
    client.close().await;
}

#[tokio::test]
#[ignore]
async fn test_get_all_states() {
    let client = make_client().await;
    let states = client.get_all_states().await;
    // Should have at least some state from SSE
    let _ = states; // May be empty if no breakers configured
    client.close().await;
}

#[tokio::test]
#[ignore]
async fn test_get_status() {
    let client = make_client().await;
    let status = client.get_status().await.unwrap();
    assert!(status.open_count >= 0);
    assert!(status.closed_count >= 0);
    client.close().await;
}

#[tokio::test]
#[ignore]
async fn test_execute() {
    let client = make_client().await;

    let result: Result<String, std::io::Error> = client
        .execute(
            || async { Ok("hello".to_string()) },
            ExecuteOptions::new().metric("latency", MetricValue::Latency),
        )
        .await
        .map_err(|e| match e {
            tripswitch::ExecuteError::Sdk(e) => std::io::Error::other(e.to_string()),
            tripswitch::ExecuteError::Task(e) => e,
        });

    assert_eq!(result.unwrap(), "hello");
    client.close().await;
}

#[tokio::test]
#[ignore]
async fn test_stats() {
    let client = make_client().await;
    let stats = client.stats();
    assert_eq!(stats.dropped_samples, 0);
    assert!(stats.buffer_capacity > 0);
    client.close().await;
}

#[tokio::test]
#[ignore]
async fn test_graceful_shutdown() {
    let client = make_client().await;
    assert!(client.stats().sse_connected);
    client.close().await;
    // After close, the client should have cancelled all tasks
}

#[tokio::test]
#[ignore]
async fn test_metadata_sync() {
    let client = make_client().await;
    // Give metadata sync a moment to run
    tokio::time::sleep(Duration::from_secs(2)).await;
    // Metadata may or may not be available depending on project config
    let _breakers = client.get_breakers_metadata();
    let _routers = client.get_routers_metadata();
    client.close().await;
}
