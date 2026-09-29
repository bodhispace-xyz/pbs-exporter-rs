//! Integration tests for PBS exporter
//!
//! These tests use mockito to simulate PBS API responses

use mockito::Server;
use pbs_exporter::{client::PbsClient, config::PbsConfig, metrics::MetricsCollector};

/// Helper to create a test PBS config pointing to mock server
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

#[tokio::test]
async fn test_node_status_success() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/api2/json/nodes/localhost/status")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "data": {
                "cpu": 0.25,
                "wait": 0.01,
                "memory": {
                    "used": 8589934592,
                    "total": 17179869184,
                    "free": 8589934592
                },
                "swap": {
                    "used": 0,
                    "total": 4294967296,
                    "free": 4294967296
                },
                "root": {
                    "used": 53687091200,
                    "total": 107374182400,
                    "avail": 53687091200
                },
                "loadavg": [0.5, 0.4, 0.3],
                "uptime": 86400
            }
        }"#,
        )
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();

    let status = client.get_node_status().await.unwrap();

    assert_eq!(status.cpu, 0.25);
    assert_eq!(status.wait, 0.01);
    assert_eq!(status.memory.used, 8589934592);
    assert_eq!(status.memory.total, 17179869184);
    assert_eq!(status.uptime, 86400);
    assert_eq!(status.loadavg, [0.5, 0.4, 0.3]);

    mock.assert_async().await;
}

#[tokio::test]
async fn test_datastore_usage_success() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/api2/json/status/datastore-usage")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "data": [
                {
                    "store": "datastore1",
                    "total": 1099511627776,
                    "used": 549755813888,
                    "avail": 549755813888
                },
                {
                    "store": "datastore2",
                    "total": 2199023255552,
                    "used": 1099511627776,
                    "avail": 1099511627776
                }
            ]
        }"#,
        )
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();

    let datastores = client.get_datastore_usage().await.unwrap();

    assert_eq!(datastores.len(), 2);
    assert_eq!(datastores[0].store, "datastore1");
    assert_eq!(datastores[0].total, 1099511627776);
    assert_eq!(datastores[0].used, 549755813888);
    assert_eq!(datastores[1].store, "datastore2");

    mock.assert_async().await;
}

#[tokio::test]
async fn test_backup_groups_success() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/api2/json/admin/datastore/datastore1/groups")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "data": [
                {
                    "backup-type": "vm",
                    "backup-id": "100",
                    "backup-count": 7,
                    "last-backup": 1703635200
                },
                {
                    "backup-type": "ct",
                    "backup-id": "101",
                    "backup-count": 5,
                    "last-backup": 1703721600
                }
            ]
        }"#,
        )
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();

    let groups = client.get_backup_groups("datastore1").await.unwrap();

    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].backup_type, "vm");
    assert_eq!(groups[0].backup_id, "100");
    assert_eq!(groups[0].backup_count, 7);
    assert_eq!(groups[0].last_backup, 1703635200);
    assert_eq!(groups[1].backup_type, "ct");
    assert_eq!(groups[1].backup_id, "101");

    mock.assert_async().await;
}

#[tokio::test]
async fn test_version_success() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/api2/json/version")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
            "data": {
                "version": "4.1.1",
                "release": "1",
                "repoid": "abc123def"
            }
        }"#,
        )
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();

    let version = client.get_version().await.unwrap();

    assert_eq!(version.version, "4.1.1");
    assert_eq!(version.release, "1");
    assert_eq!(version.repoid, "abc123def");

    mock.assert_async().await;
}

#[tokio::test]
async fn test_api_error_handling() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/api2/json/nodes/localhost/status")
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "authentication failed"}"#)
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();

    let result = client.get_node_status().await;

    assert!(result.is_err());
    mock.assert_async().await;
}

#[tokio::test]
async fn test_metrics_collection_success() {
    let mut server = Server::new_async().await;

    // Mock node status
    let _mock_status = server
        .mock("GET", "/api2/json/nodes/localhost/status")
        .with_status(200)
        .with_body(
            r#"{
            "data": {
                "cpu": 0.15,
                "wait": 0.02,
                "memory": {"used": 4294967296, "total": 8589934592, "free": 4294967296},
                "swap": {"used": 0, "total": 2147483648, "free": 2147483648},
                "root": {"used": 10737418240, "total": 53687091200, "avail": 42949672960},
                "loadavg": [0.3, 0.2, 0.1],
                "uptime": 3600
            }
        }"#,
        )
        .create_async()
        .await;

    // Mock datastore usage
    let _mock_datastores = server
        .mock("GET", "/api2/json/status/datastore-usage")
        .with_status(200)
        .with_body(r#"{
            "data": [
                {"store": "backup", "total": 1099511627776, "used": 549755813888, "avail": 549755813888}
            ]
        }"#)
        .create_async()
        .await;

    let _mock_namespaces = server
        .mock("GET", "/api2/json/admin/datastore/backup/namespace")
        .with_status(200)
        .with_body(r#"{"data": [{"ns": ""}]}"#)
        .create_async()
        .await;

    // Mock backup groups
    let _mock_groups = server
        .mock("GET", "/api2/json/admin/datastore/backup/groups")
        .with_status(200)
        .with_body(r#"{
            "data": [
                {"backup-type": "vm", "backup-id": "100", "backup-count": 3, "last-backup": 1703635200}
            ]
        }"#)
        .create_async()
        .await;

    // Mock version
    let _mock_version = server
        .mock("GET", "/api2/json/version")
        .with_status(200)
        .with_body(
            r#"{
            "data": {"version": "4.1.0", "release": "1", "repoid": "test"}
        }"#,
        )
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();
    let collector = MetricsCollector::new(std::sync::Arc::new(client), 0).unwrap();

    // Collect metrics
    let result = collector.collect().await;
    assert!(result.is_ok());

    // Encode metrics
    let metrics_output = collector.encode().unwrap();

    // Verify key metrics are present
    assert!(metrics_output.contains("pbs_up 1"));
    assert!(metrics_output.contains("pbs_host_cpu_usage"));
    assert!(metrics_output.contains("pbs_datastore_total_bytes"));
    assert!(metrics_output.contains("pbs_snapshot_count"));
    assert!(metrics_output.contains("pbs_version"));
}

#[tokio::test]
async fn test_metrics_collection_failure() {
    let mut server = Server::new_async().await;

    // Mock failed node status
    let _mock = server
        .mock("GET", "/api2/json/nodes/localhost/status")
        .with_status(500)
        .create_async()
        .await;

    let config = create_test_config(&server.url());
    let client = PbsClient::new(config).unwrap();
    let collector = MetricsCollector::new(std::sync::Arc::new(client), 0).unwrap();

    // Collection should fail but not panic
    let result = collector.collect().await;
    assert!(result.is_err());

    // Should still be able to encode (with pbs_up = 0)
    let metrics_output = collector.encode().unwrap();
    assert!(metrics_output.contains("pbs_up 0"));
}
