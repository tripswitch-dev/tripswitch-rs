use tripswitch::admin::{errors::AdminError, types::*, AdminClient};

fn admin_client() -> AdminClient {
    let api_key = std::env::var("TRIPSWITCH_API_KEY").expect("TRIPSWITCH_API_KEY must be set");
    let mut builder = AdminClient::builder(api_key);
    if let Ok(url) = std::env::var("TRIPSWITCH_BASE_URL") {
        builder = builder.base_url(url);
    }
    builder.build()
}

fn bad_client() -> AdminClient {
    let mut builder = AdminClient::builder("eb_admin_invalid".to_string());
    if let Ok(url) = std::env::var("TRIPSWITCH_BASE_URL") {
        builder = builder.base_url(url);
    }
    builder.build()
}

fn project_id() -> String {
    std::env::var("TRIPSWITCH_PROJECT_ID").expect("TRIPSWITCH_PROJECT_ID must be set")
}

fn workspace_id() -> Option<String> {
    std::env::var("TRIPSWITCH_WORKSPACE_ID").ok()
}

// ── Projects ─────────────────────────────────────────────────────────

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
async fn test_list_projects() {
    let client = admin_client();
    let pid = project_id();
    let resp = client.list_projects().await.unwrap();
    assert!(resp.projects.iter().any(|p| p.id == pid));
}

#[tokio::test]
#[ignore]
async fn test_project_crud() {
    let client = admin_client();
    let name = format!("integration-test-rs-{}", chrono::Utc::now().timestamp());

    let input = CreateProjectInput {
        name: name.clone(),
        workspace_id: workspace_id(),
    };
    let project = client.create_project(&input).await.unwrap();
    assert_eq!(project.name.as_deref(), Some(name.as_str()));

    // List — should appear
    let resp = client.list_projects().await.unwrap();
    assert!(resp.projects.iter().any(|p| p.id == project.id));

    // Delete
    client.delete_project(&project.id, &name).await.unwrap();

    // Verify gone
    let result = client.get_project(&project.id).await;
    assert!(result.is_err());
}

// ── Breakers ─────────────────────────────────────────────────────────

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
async fn test_breaker_crud() {
    let client = admin_client();
    let pid = project_id();

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

    let fetched = client.get_breaker(&pid, &breaker.id).await.unwrap();
    assert_eq!(fetched.id, breaker.id);

    let update = UpdateBreakerInput {
        threshold: Some(0.8),
        name: None, metric: None, kind: None, kind_params: None, op: None,
        window_ms: None, min_count: None, min_state_duration_ms: None,
        cooldown_ms: None, eval_interval_ms: None, half_open_backoff_enabled: None,
        half_open_backoff_cap_ms: None, half_open_indeterminate_policy: None,
        recovery_allow_rate_ramp_steps: None, actions: None, metadata: None,
    };
    let updated = client.update_breaker(&pid, &breaker.id, &update).await.unwrap();
    assert!((updated.threshold - 0.8).abs() < f64::EPSILON);

    client.delete_breaker(&pid, &breaker.id).await.unwrap();

    let result = client.get_breaker(&pid, &breaker.id).await;
    assert!(result.is_err());
}

// ── Routers ───────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn test_list_routers() {
    let client = admin_client();
    let pid = project_id();
    let resp = client.list_routers(&pid, None).await.unwrap();
    let _ = resp.routers;
}

// ── Notification Channels ─────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn test_list_notification_channels() {
    let client = admin_client();
    let pid = project_id();
    let resp = client.list_notification_channels(&pid, None).await.unwrap();
    let _ = resp.channels;
}

// ── Events ────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn test_list_events() {
    let client = admin_client();
    let pid = project_id();
    let resp = client.list_events(&pid, None).await.unwrap();
    assert!(resp.returned >= 0);
}

// ── Project Keys ──────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn test_list_project_keys() {
    let client = admin_client();
    let pid = project_id();
    let resp = client.list_project_keys(&pid).await.unwrap();
    let _ = resp.keys;
}

// ── Workspaces ────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn test_get_workspace() {
    let wid = match workspace_id() {
        Some(id) => id,
        None => return, // skip if not configured
    };
    let client = admin_client();
    let ws = client.get_workspace(&wid).await.unwrap();
    assert_eq!(ws.id, wid);
}

// ── Error Handling ────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn test_not_found_error() {
    let client = admin_client();
    let err = client
        .get_project("00000000-0000-0000-0000-000000000000")
        .await
        .unwrap_err();
    assert!(err.is_not_found(), "expected not_found, got: {err:?}");
}

#[tokio::test]
#[ignore]
async fn test_unauthorized_error() {
    let client = bad_client();
    let err = client.get_project("any").await.unwrap_err();
    assert!(
        matches!(err, AdminError::Unauthorized(_)),
        "expected unauthorized, got: {err:?}"
    );
}
