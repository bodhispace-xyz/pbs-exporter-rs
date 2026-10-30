//! HTTP server endpoint tests

use mockito::Server;
use pbs_exporter::{
    client::PbsClient, config::PbsConfig, metrics::MetricsCollector, server::start_server,
};
use std::time::Duration;

/// Helper to create test config
fn create_test_config(server_url: &str) -> PbsConfig {
    PbsConfig {
        endpoint: server_url.to_string(),
        token_id: "test@pam!token".to_string(),
        token_secret: "test-secret".to_string(),
        verify_tls: false,
        timeout_seconds: 5,
        snapshot_history_limit: 0,
    }
}

// WGT: Test health endpoint is accessible
#[tokio::test]
async fn test_health_endpoint() {
    // Given: A mock PBS server with minimal status response
    let mut server = Server::new_async().await;

    let _mock_status = server
        .mock("GET", "/api2/json/nodes/localhost/status")
        .with_status(200)
        .with_body(r#"{"data": {"cpu": 0.1, "wait": 0.01, "memory": {"used": 1000, "total": 2000, "free": 1000}, "swap": {"used": 0, "total": 1000, "free": 1000}, "root": {"used": 1000, "total": 2000, "avail": 1000}, "loadavg": [0.1, 0.1, 0.1], "uptime": 100}}"#)
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();
    let collector = MetricsCollector::new(std::sync::Arc::new(client), 0).unwrap();

    // When: Starting the server in background
    let server_handle = tokio::spawn(async move { start_server("127.0.0.1:0", collector).await });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Then: Server should start successfully (placeholder for actual HTTP client test)
    // In a real scenario, you'd use reqwest to test the endpoints

    // Cleanup
    server_handle.abort();
}

// WGT: Test metrics endpoint returns valid Prometheus format
#[tokio::test]
async fn test_metrics_endpoint_returns_prometheus_format() {
    // Given: A mock PBS server with complete endpoint responses
    let mut server = Server::new_async().await;

    let _mock_status = server
        .mock("GET", "/api2/json/nodes/localhost/status")
        .with_status(200)
        .with_body(r#"{"data": {"cpu": 0.25, "wait": 0.02, "memory": {"used": 8589934592, "total": 17179869184, "free": 8589934592}, "swap": {"used": 0, "total": 4294967296, "free": 4294967296}, "root": {"used": 10737418240, "total": 53687091200, "avail": 42949672960}, "loadavg": [0.5, 0.4, 0.3], "uptime": 3600}}"#)
        .create_async()
        .await;

    let _mock_datastores = server
        .mock("GET", "/api2/json/status/datastore-usage")
        .with_status(200)
        .with_body(r#"{"data": [{"store": "backup", "total": 1099511627776, "used": 549755813888, "avail": 549755813888}]}"#)
        .create_async()
        .await;

    let _mock_namespaces = server
        .mock("GET", "/api2/json/admin/datastore/backup/namespace")
        .with_status(200)
        .with_body(r#"{"data": [{"ns": ""}]}"#)
        .create_async()
        .await;

    let _mock_groups = server
        .mock("GET", "/api2/json/admin/datastore/backup/groups")
        .with_status(200)
        .with_body(r#"{"data": [{"backup-type": "vm", "backup-id": "100", "backup-count": 3, "last-backup": 1703635200}]}"#)
        .create_async()
        .await;

    let _mock_version = server
        .mock("GET", "/api2/json/version")
        .with_status(200)
        .with_body(r#"{"data": {"version": "4.1.0", "release": "1", "repoid": "test"}}"#)
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();
    let collector = MetricsCollector::new(std::sync::Arc::new(client), 0).unwrap();

    // When: Collecting and encoding metrics
    collector.collect().await.unwrap();
    let metrics = collector.encode().unwrap();

    // Then: Metrics should be in valid Prometheus format with expected content
    assert!(metrics.contains("# HELP"));
    assert!(metrics.contains("# TYPE"));
    assert!(metrics.contains("pbs_up 1"));
    assert!(metrics.contains("pbs_host_cpu_usage"));
    assert!(metrics.contains("pbs_datastore_total_bytes"));

    // Verify metric format (name{labels} value)
    assert!(metrics.contains(r#"pbs_datastore_total_bytes{datastore="backup"}"#));
    // Note: Prometheus format has spaces after commas in labels
    assert!(metrics.contains(r#"backup_id="100""#));
    assert!(metrics.contains(r#"backup_type="vm""#));
}

// WGT: Test edge case with no datastores configured
#[tokio::test]
async fn test_edge_case_empty_datastores() {
    // Given: A mock PBS server with no datastores configured
    let mut server = Server::new_async().await;

    let _mock_status = server
        .mock("GET", "/api2/json/nodes/localhost/status")
        .with_status(200)
        .with_body(r#"{"data": {"cpu": 0.1, "wait": 0.01, "memory": {"used": 1000, "total": 2000, "free": 1000}, "swap": {"used": 0, "total": 1000, "free": 1000}, "root": {"used": 1000, "total": 2000, "avail": 1000}, "loadavg": [0.1, 0.1, 0.1], "uptime": 100}}"#)
        .create_async()
        .await;

    let _mock_datastores = server
        .mock("GET", "/api2/json/status/datastore-usage")
        .with_status(200)
        .with_body(r#"{"data": []}"#)
        .create_async()
        .await;

    let _mock_version = server
        .mock("GET", "/api2/json/version")
        .with_status(200)
        .with_body(r#"{"data": {"version": "4.1.0", "release": "1", "repoid": "test"}}"#)
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();
    let collector = MetricsCollector::new(std::sync::Arc::new(client), 0).unwrap();

    // When: Collecting metrics with empty datastore list
    let result = collector.collect().await;

    // Then: Collection should succeed without panicking
    assert!(result.is_ok());

    let metrics = collector.encode().unwrap();
    assert!(metrics.contains("pbs_up 1"));
    assert!(metrics.contains("pbs_host_cpu_usage"));
}

// WGT: Test edge case with datastore having no backup groups
#[tokio::test]
async fn test_edge_case_empty_backup_groups() {
    // Given: A mock PBS server with a datastore that has no backup groups
    let mut server = Server::new_async().await;

    let _mock_status = server
        .mock("GET", "/api2/json/nodes/localhost/status")
        .with_status(200)
        .with_body(r#"{"data": {"cpu": 0.1, "wait": 0.01, "memory": {"used": 1000, "total": 2000, "free": 1000}, "swap": {"used": 0, "total": 1000, "free": 1000}, "root": {"used": 1000, "total": 2000, "avail": 1000}, "loadavg": [0.1, 0.1, 0.1], "uptime": 100}}"#)
        .create_async()
        .await;

    let _mock_datastores = server
        .mock("GET", "/api2/json/status/datastore-usage")
        .with_status(200)
        .with_body(r#"{"data": [{"store": "empty-store", "total": 1000000, "used": 0, "avail": 1000000}]}"#)
        .create_async()
        .await;

    let _mock_groups = server
        .mock("GET", "/api2/json/admin/datastore/empty-store/groups")
        .with_status(200)
        .with_body(r#"{"data": []}"#)
        .create_async()
        .await;

    let _mock_version = server
        .mock("GET", "/api2/json/version")
        .with_status(200)
        .with_body(r#"{"data": {"version": "4.1.0", "release": "1", "repoid": "test"}}"#)
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();
    let collector = MetricsCollector::new(std::sync::Arc::new(client), 0).unwrap();

    // When: Collecting metrics from empty datastore
    let result = collector.collect().await;

    // Then: Collection should succeed and include datastore capacity metrics
    assert!(result.is_ok());

    let metrics = collector.encode().unwrap();
    assert!(metrics.contains("pbs_up 1"));
    assert!(metrics.contains(r#"pbs_datastore_total_bytes{datastore="empty-store"}"#));
}

// WGT: Test datastore names with special characters are handled correctly
#[tokio::test]
async fn test_edge_case_special_characters_in_datastore_name() {
    // Given: A mock PBS server with datastore name containing special characters
    let mut server = Server::new_async().await;

    let _mock_status = server
        .mock("GET", "/api2/json/nodes/localhost/status")
        .with_status(200)
        .with_body(r#"{"data": {"cpu": 0.1, "wait": 0.01, "memory": {"used": 1000, "total": 2000, "free": 1000}, "swap": {"used": 0, "total": 1000, "free": 1000}, "root": {"used": 1000, "total": 2000, "avail": 1000}, "loadavg": [0.1, 0.1, 0.1], "uptime": 100}}"#)
        .create_async()
        .await;

    let _mock_datastores = server
        .mock("GET", "/api2/json/status/datastore-usage")
        .with_status(200)
        .with_body(r#"{"data": [{"store": "backup-2024", "total": 1000000, "used": 500000, "avail": 500000}]}"#)
        .create_async()
        .await;

    let _mock_groups = server
        .mock("GET", "/api2/json/admin/datastore/backup-2024/groups")
        .with_status(200)
        .with_body(r#"{"data": []}"#)
        .create_async()
        .await;

    let _mock_version = server
        .mock("GET", "/api2/json/version")
        .with_status(200)
        .with_body(r#"{"data": {"version": "4.1.0", "release": "1", "repoid": "test"}}"#)
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();
    let collector = MetricsCollector::new(std::sync::Arc::new(client), 0).unwrap();

    // When: Collecting metrics with special characters in datastore name
    let result = collector.collect().await;

    // Then: Collection should succeed and properly handle the datastore name
    assert!(result.is_ok());

    let metrics = collector.encode().unwrap();
    assert!(metrics.contains(r#"datastore="backup-2024""#));
}

// WGT: Test partial API failure doesn't stop entire collection
#[tokio::test]
async fn test_partial_failure_continues_collection() {
    // Given: A mock PBS server with two datastores where one fails with 403
    let mut server = Server::new_async().await;

    let _mock_status = server
        .mock("GET", "/api2/json/nodes/localhost/status")
        .with_status(200)
        .with_body(r#"{"data": {"cpu": 0.1, "wait": 0.01, "memory": {"used": 1000, "total": 2000, "free": 1000}, "swap": {"used": 0, "total": 1000, "free": 1000}, "root": {"used": 1000, "total": 2000, "avail": 1000}, "loadavg": [0.1, 0.1, 0.1], "uptime": 100}}"#)
        .create_async()
        .await;

    let _mock_datastores = server
        .mock("GET", "/api2/json/status/datastore-usage")
        .with_status(200)
        .with_body(r#"{"data": [{"store": "store1", "total": 1000000, "used": 500000, "avail": 500000}, {"store": "store2", "total": 2000000, "used": 1000000, "avail": 1000000}]}"#)
        .create_async()
        .await;

    let _mock_groups1 = server
        .mock("GET", "/api2/json/admin/datastore/store1/groups")
        .with_status(200)
        .with_body(r#"{"data": [{"backup-type": "vm", "backup-id": "100", "backup-count": 5, "last-backup": 1703635200}]}"#)
        .create_async()
        .await;

    let _mock_groups2 = server
        .mock("GET", "/api2/json/admin/datastore/store2/groups")
        .with_status(403)
        .with_body(r#"{"error": "forbidden"}"#)
        .create_async()
        .await;

    let _mock_version = server
        .mock("GET", "/api2/json/version")
        .with_status(200)
        .with_body(r#"{"data": {"version": "4.1.0", "release": "1", "repoid": "test"}}"#)
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();
    let collector = MetricsCollector::new(std::sync::Arc::new(client), 0).unwrap();

    // When: Collecting metrics with one datastore failing
    let result = collector.collect().await;

    // Then: Overall collection should succeed with partial results
    assert!(result.is_ok());

    let metrics = collector.encode().unwrap();
    assert!(metrics.contains("pbs_up 1"));
    // Should have metrics for store1
    assert!(metrics.contains(r#"datastore="store1""#));
    // Should have metrics for store2 capacity (but not groups)
    assert!(metrics.contains(r#"datastore="store2""#));
}

// WGT: Test performance with large number of VMs (100+)
#[tokio::test]
async fn test_large_number_of_vms() {
    // Given: A mock PBS server with 100 VMs (IDs 100-199)
    let mut server = Server::new_async().await;

    let _mock_status = server
        .mock("GET", "/api2/json/nodes/localhost/status")
        .with_status(200)
        .with_body(r#"{"data": {"cpu": 0.1, "wait": 0.01, "memory": {"used": 1000, "total": 2000, "free": 1000}, "swap": {"used": 0, "total": 1000, "free": 1000}, "root": {"used": 1000, "total": 2000, "avail": 1000}, "loadavg": [0.1, 0.1, 0.1], "uptime": 100}}"#)
        .create_async()
        .await;

    let _mock_datastores = server
        .mock("GET", "/api2/json/status/datastore-usage")
        .with_status(200)
        .with_body(r#"{"data": [{"store": "large", "total": 1000000000, "used": 500000000, "avail": 500000000}]}"#)
        .create_async()
        .await;

    let _mock_namespaces = server
        .mock("GET", "/api2/json/admin/datastore/large/namespace")
        .with_status(200)
        .with_body(r#"{"data": [{"ns": ""}]}"#)
        .create_async()
        .await;


    let mut groups = Vec::new();
    for i in 100..200 {
        groups.push(format!(
            r#"{{"backup-type": "vm", "backup-id": "{}", "backup-count": 7, "last-backup": 1703635200}}"#,
            i
        ));
    }
    let groups_json = format!(r#"{{"data": [{}]}}"#, groups.join(","));

    let _mock_groups = server
        .mock("GET", "/api2/json/admin/datastore/large/groups")
        .with_status(200)
        .with_body(&groups_json)
        .create_async()
        .await;

    let _mock_version = server
        .mock("GET", "/api2/json/version")
        .with_status(200)
        .with_body(r#"{"data": {"version": "4.1.0", "release": "1", "repoid": "test"}}"#)
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();
    let collector = MetricsCollector::new(std::sync::Arc::new(client), 0).unwrap();

    // When: Collecting metrics for large number of VMs
    let result = collector.collect().await;

    // Then: Collection should succeed and include metrics for all VMs
    assert!(result.is_ok());

    let metrics = collector.encode().unwrap();
    assert!(metrics.contains("pbs_up 1"));

    // Should have metrics for all 100 VMs
    assert!(metrics.contains(r#"backup_id="100""#));
    assert!(metrics.contains(r#"backup_id="199""#));
}
