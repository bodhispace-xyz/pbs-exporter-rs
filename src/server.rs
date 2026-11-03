//! HTTP server for exposing Prometheus metrics.
//!
//! This module provides an Axum-based HTTP server that serves the `/metrics`
//! endpoint for Prometheus scraping and a `/health` endpoint for health checks.

use crate::error::Result;
use crate::metrics::MetricsCollector;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

/// Shared application state.
#[derive(Clone)]
struct AppState {
    metrics: Arc<MetricsCollector>,
}

/// Start the HTTP server for serving Prometheus metrics.
///
/// Creates and starts an Axum-based HTTP server that exposes three endpoints:
/// - `/metrics` - Prometheus metrics endpoint (scrapes PBS on each request)
/// - `/health` - Health check endpoint (always returns OK)
/// - `/` - Root endpoint with HTML information page
///
/// The server runs indefinitely until the process is terminated or an error occurs.
///
/// # Arguments
///
/// * `listen_address` - Address to bind to (e.g., "0.0.0.0:9101" or "127.0.0.1:9101")
/// * `metrics` - Metrics collector instance that will be used to gather PBS metrics
///
/// # Returns
///
/// Returns `Ok(())` if the server starts successfully and runs until shutdown,
/// or an error if server startup or operation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The listen address is invalid or cannot be parsed
/// - The port is already in use or cannot be bound
/// - A network error occurs during server operation
///
/// # Example
///
/// ```no_run
/// use pbs_exporter::server::start_server;
/// use pbs_exporter::metrics::MetricsCollector;
/// use pbs_exporter::client::PbsClient;
/// use pbs_exporter::config::PbsConfig;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = PbsConfig {
///         endpoint: "https://pbs.example.com:8007".to_string(),
///         token_id: "user@pam!token".to_string(),
///         token_secret: "secret".to_string(),
///         verify_tls: true,
///         ca_cert_path: None,
///         timeout_seconds: 5,
///         snapshot_history_limit: 0,
///     };
///     let client = PbsClient::new(config)?;
///     let metrics = MetricsCollector::new(std::sync::Arc::new(client), 0)?;
///     start_server("0.0.0.0:9101", metrics).await?;
///     Ok(())
/// }
/// ```
pub async fn start_server(listen_address: &str, metrics: MetricsCollector) -> Result<()> {
    let app = router(metrics);

    info!("Routes configured: /metrics, /health, /");
    info!("Starting HTTP server on {}", listen_address);

    let listener = TcpListener::bind(listen_address).await?;
    info!(
        "Successfully bound to {}, server is ready to accept connections",
        listen_address
    );

    axum::serve(listener, app.into_make_service())
        .await
        .map_err(|e| crate::error::PbsError::Server(e.to_string()))?;

    Ok(())
}

fn router(metrics: MetricsCollector) -> Router {
    let state = AppState {
        metrics: Arc::new(metrics),
    };

    Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler))
        .route("/", get(root_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Handler for /metrics endpoint.
async fn metrics_handler(State(state): State<AppState>) -> Response {
    info!("=== METRICS HANDLER CALLED ===");
    info!("Received metrics scrape request");

    // Capture this scrape before another request can reset the shared registry.
    match state.metrics.collect_and_encode().await {
        Ok(body) => (StatusCode::OK, body).into_response(),
        Err(e) => {
            warn!("Failed to encode metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to encode metrics: {}", e),
            )
                .into_response()
        }
    }
}

/// Handler for /health endpoint.
async fn health_handler() -> Response {
    info!("=== HEALTH HANDLER CALLED ===");
    (StatusCode::OK, "OK").into_response()
}

/// Handler for root endpoint.
async fn root_handler() -> Response {
    info!("=== ROOT HANDLER CALLED ===");
    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>PBS Exporter</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        h1 { color: #333; }
        a { color: #0066cc; text-decoration: none; }
        a:hover { text-decoration: underline; }
        .info { background: #f0f0f0; padding: 15px; border-radius: 5px; margin: 20px 0; }
    </style>
</head>
<body>
    <h1>PBS Exporter</h1>
    <div class="info">
        <p>Prometheus metrics exporter for Proxmox Backup Server 4.x</p>
        <p><strong>Endpoints:</strong></p>
        <ul>
            <li><a href="/metrics">/metrics</a> - Prometheus metrics</li>
            <li><a href="/health">/health</a> - Health check</li>
        </ul>
    </div>
    <p>
        <a href="https://github.com/yourusername/pbs-exporter-rs">GitHub Repository</a>
    </p>
</body>
</html>
"#;

    (StatusCode::OK, html).into_response()
}

#[cfg(test)]
mod hardening_tests {
    use super::*;
    use crate::{client::PbsClient, config::PbsConfig};
    use axum::{
        extract::Request,
        middleware::{self, Next},
    };
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };
    use tokio::{sync::Notify, task::JoinHandle};

    #[derive(Default)]
    struct Fixture {
        nodes: AtomicUsize,
        tasks: AtomicUsize,
        first_tasks_entered: Notify,
        release_first_tasks: Notify,
        second_node_entered: Notify,
        release_second_node: Notify,
    }

    struct TestServer {
        url: String,
        task: JoinHandle<()>,
    }
    impl Drop for TestServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    async fn serve(app: Router) -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        TestServer { url, task }
    }

    async fn node(State(fixture): State<Arc<Fixture>>) -> String {
        let index = fixture.nodes.fetch_add(1, Ordering::SeqCst);
        if index == 1 {
            fixture.second_node_entered.notify_one();
            fixture.release_second_node.notified().await;
        }
        let cpu = if index == 0 { "0.25" } else { "0.75" };
        r#"{"data":{"cpu":CPU,"wait":0,"memory":{"used":1,"total":2,"free":1},"swap":{"used":0,"total":2,"free":2},"root":{"used":1,"total":2,"avail":1},"loadavg":[0,0,0],"uptime":100}}"#.replace("CPU", cpu)
    }

    async fn tasks(State(fixture): State<Arc<Fixture>>) -> &'static str {
        if fixture.tasks.fetch_add(1, Ordering::SeqCst) == 0 {
            fixture.first_tasks_entered.notify_one();
            fixture.release_first_tasks.notified().await;
        }
        r#"{"data":[]}"#
    }

    async fn overlapping_scrapes() {
        let fixture = Arc::new(Fixture::default());
        let pbs = serve(Router::new()
            .route("/api2/json/nodes/localhost/status", get(node))
            .route("/api2/json/nodes/localhost/tasks", get(tasks))
            .route("/api2/json/status/datastore-usage", get(|| async { r#"{"data":[{"store":"backup","total":100,"used":50,"avail":50}]}"# }))
            .route("/api2/json/admin/datastore/backup/namespace", get(|| async { r#"{"data":[{"ns":""}]}"# }))
            .route("/api2/json/admin/datastore/backup/snapshots", get(|| async { r#"{"data":[{"backup-type":"vm","backup-id":"100","backup-time":1000,"comment":"dummy"}]}"# }))
            .route("/api2/json/admin/datastore/backup/groups", get(|| async { r#"{"data":[{"backup-type":"vm","backup-id":"100","backup-count":1,"last-backup":1000}]}"# }))
            .route("/api2/json/admin/datastore/backup/gc", get(|| async { r#"{"data":{}}"# }))
            .route("/api2/json/tape/drive", get(|| async { r#"{"data":[]}"# }))
            .route("/api2/json/version", get(|| async { r#"{"data":{"version":"4.1.0","release":"test","repoid":"test"}}"# }))
            .with_state(fixture.clone())).await;
        let client = PbsClient::new(PbsConfig {
            endpoint: pbs.url.clone(),
            token_id: "dummy@pam!token".into(),
            token_secret: "dummy".into(),
            verify_tls: false,
            ca_cert_path: None,
            timeout_seconds: 10,
            snapshot_history_limit: 0,
        })
        .unwrap();
        let collector = MetricsCollector::new(Arc::new(client), 0).unwrap();
        let arrivals = Arc::new(AtomicUsize::new(0));
        let second_arrived = Arc::new(Notify::new());
        let arrived = second_arrived.clone();
        let app = router(collector).layer(middleware::from_fn(move |req: Request, next: Next| {
            let arrivals = arrivals.clone();
            let arrived = arrived.clone();
            async move {
                if req.uri().path() == "/metrics" && arrivals.fetch_add(1, Ordering::SeqCst) == 1 {
                    arrived.notify_one();
                }
                next.run(req).await
            }
        }));
        let exporter = serve(app).await;
        let http = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        let first_url = format!("{}/metrics", exporter.url);
        let first_http = http.clone();
        let first = tokio::spawn(async move {
            first_http
                .get(first_url)
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        });
        fixture.first_tasks_entered.notified().await;
        assert_eq!(
            http.get(format!("{}/health", exporter.url))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        let second_url = format!("{}/metrics", exporter.url);
        let second_http = http.clone();
        let second = tokio::spawn(async move {
            second_http
                .get(second_url)
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        });
        second_arrived.notified().await;
        // The second HTTP request has arrived, but must not reset state or contact PBS
        // until the first response has been encoded. A bounded negative wait checks this.
        let overlapped = tokio::time::timeout(
            Duration::from_millis(300),
            fixture.second_node_entered.notified(),
        )
        .await
        .is_ok();
        fixture.release_first_tasks.notify_one();
        let first_body = first.await.unwrap();
        fixture.release_second_node.notify_one();
        let second_body = second.await.unwrap();
        assert!(
            !overlapped,
            "second scrape entered PBS while first collection was paused"
        );
        for (body, cpu) in [(&first_body, "0.25"), (&second_body, "0.75")] {
            assert!(body.contains("pbs_up 1"));
            assert!(
                body.contains(&format!("pbs_host_cpu_usage {cpu}")),
                "{body}"
            );
            assert!(body.contains("pbs_snapshot_count{"), "{body}");
        }
    }

    #[tokio::test]
    async fn collection_failure_still_serves_metrics_with_pbs_down() {
        let pbs = serve(Router::new().route(
            "/api2/json/nodes/localhost/status",
            get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
        ))
        .await;
        let mut config = crate::config::Settings::default().pbs;
        config.endpoint = pbs.url.clone();
        config.verify_tls = false;
        config.token_id = "dummy@pam!token".into();
        config.token_secret = "dummy".into();
        let collector =
            MetricsCollector::new(Arc::new(PbsClient::new(config).unwrap()), 0).unwrap();
        let exporter = serve(router(collector)).await;
        let response = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap()
            .get(format!("{}/metrics", exporter.url))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.text().await.unwrap().contains("pbs_up 0"));
    }

    #[tokio::test]
    async fn concurrent_http_scrapes_keep_complete_independent_responses() {
        tokio::time::timeout(Duration::from_secs(15), overlapping_scrapes())
            .await
            .unwrap();
    }
}
