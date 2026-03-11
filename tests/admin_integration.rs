use tripswitch::admin::{AdminClient, types::*};

fn admin_client() -> AdminClient {
    let api_key = std::env::var("TRIPSWITCH_API_KEY")
        .expect("TRIPSWITCH_API_KEY must be set");
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
    let project = client.get_project(&pid, None).await.unwrap();
    assert_eq!(project.id, pid);
}

#[tokio::test]
#[ignore]
async fn test_list_breakers() {
    let client = admin_client();
    let pid = project_id();
    let resp = client.list_breakers(&pid, None, None).await.unwrap();
    assert!(resp.total >= 0);
}

#[tokio::test]
#[ignore]
async fn test_list_routers() {
    let client = admin_client();
    let pid = project_id();
    let resp = client.list_routers(&pid, None, None).await.unwrap();
    assert!(resp.total >= 0);
}

#[tokio::test]
#[ignore]
async fn test_list_events() {
    let client = admin_client();
    let pid = project_id();
    let resp = client.list_events(&pid, None, None).await.unwrap();
    assert!(resp.total >= 0);
}

#[tokio::test]
#[ignore]
async fn test_list_notification_channels() {
    let client = admin_client();
    let pid = project_id();
    let resp = client
        .list_notification_channels(&pid, None, None)
        .await
        .unwrap();
    assert!(resp.total >= 0);
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
        threshold: 0.5,
        op: BreakerOp::Gt,
        window_size: 60000,
        min_samples: 10,
        kind: None,
        description: Some("test breaker from Rust SDK".to_string()),
        half_open_policy: None,
        half_open_max_rate: None,
        cooldown: None,
        metadata: None,
    };
    let breaker = client.create_breaker(&pid, &input, None).await.unwrap();
    assert_eq!(breaker.metric, "error_rate");

    // Get
    let fetched = client.get_breaker(&pid, &breaker.id, None).await.unwrap();
    assert_eq!(fetched.id, breaker.id);

    // Update
    let update = UpdateBreakerInput {
        description: Some("updated description".to_string()),
        name: None,
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
    let updated = client
        .update_breaker(&pid, &breaker.id, &update, None)
        .await
        .unwrap();
    assert_eq!(
        updated.description.as_deref(),
        Some("updated description")
    );

    // Delete
    client
        .delete_breaker(&pid, &breaker.id, None)
        .await
        .unwrap();

    // Verify deleted
    let result = client.get_breaker(&pid, &breaker.id, None).await;
    assert!(result.is_err());
}
