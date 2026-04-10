use tripswitch::admin::{types::*, AdminClient};

fn admin_client() -> AdminClient {
    let api_key = std::env::var("TRIPSWITCH_API_KEY").expect("TRIPSWITCH_API_KEY must be set");
    let mut builder = AdminClient::builder(api_key);
    if let Ok(url) = std::env::var("TRIPSWITCH_BASE_URL") {
        builder = builder.base_url(url);
    }
    builder.build()
}

fn project_id() -> String {
    std::env::var("TRIPSWITCH_PROJECT_ID").expect("TRIPSWITCH_PROJECT_ID must be set")
}

#[tokio::test]
#[ignore]
async fn test_get_project() {
    let client = admin_client();
    let pid = project_id();
    let project = client.get_project(&pid).await.unwrap();
    assert_eq!(project.id, pid);
}

#[tokio::test]
#[ignore]
async fn test_list_breakers() {
    let client = admin_client();
    let pid = project_id();
    let resp = client.list_breakers(&pid, None).await.unwrap();
    assert!(resp.count >= 0);
}

#[tokio::test]
#[ignore]
async fn test_list_routers() {
    let client = admin_client();
    let pid = project_id();
    let resp = client.list_routers(&pid, None).await.unwrap();
    // routers response doesn't have a count field, just check it doesn't error
    let _ = resp.routers;
}

#[tokio::test]
#[ignore]
async fn test_list_events() {
    let client = admin_client();
    let pid = project_id();
    let resp = client.list_events(&pid, None).await.unwrap();
    assert!(resp.returned >= 0);
}

#[tokio::test]
#[ignore]
async fn test_list_notification_channels() {
    let client = admin_client();
    let pid = project_id();
    let resp = client.list_notification_channels(&pid, None).await.unwrap();
    // items list is present
    let _ = resp.items;
}

#[tokio::test]
#[ignore]
async fn test_breaker_crud() {
    let client = admin_client();
    let pid = project_id();

    // Create
    let input = CreateBreakerInput {
        name: format!("test-breaker-rs-{}", chrono::Utc::now().timestamp()),
        metric: "error_rate".to_string(),
        kind: BreakerKind::ErrorRate,
        kind_params: None,
        threshold: 0.5,
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
    let breaker = client.create_breaker(&pid, &input).await.unwrap();
    assert_eq!(breaker.metric, "error_rate");

    // Get
    let fetched = client.get_breaker(&pid, &breaker.id).await.unwrap();
    assert_eq!(fetched.id, breaker.id);

    // Update
    let update = UpdateBreakerInput {
        threshold: Some(0.8),
        name: None,
        metric: None,
        kind: None,
        kind_params: None,
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
    let updated = client
        .update_breaker(&pid, &breaker.id, &update)
        .await
        .unwrap();
    assert!((updated.threshold - 0.8).abs() < f64::EPSILON);

    // Delete
    client.delete_breaker(&pid, &breaker.id).await.unwrap();

    // Verify deleted
    let result = client.get_breaker(&pid, &breaker.id).await;
    assert!(result.is_err());
}
