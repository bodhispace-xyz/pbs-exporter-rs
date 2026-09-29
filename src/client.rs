//! PBS API client for communicating with Proxmox Backup Server.
//!
//! This module provides a client for interacting with the PBS REST API
//! to collect metrics data.

use crate::config::PbsConfig;
use crate::error::{PbsError, Result};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::time::Duration;
use tracing::{debug, warn};

mod types;
pub use types::*;

/// PBS API client.
#[derive(Clone)]
pub struct PbsClient {
    client: Client,
    config: PbsConfig,
    auth_header: String,
}

impl PbsClient {
    /// Create a new PBS API client.
    ///
    /// # Arguments
    ///
    /// * `config` - PBS configuration
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pbs_exporter::client::PbsClient;
    /// use pbs_exporter::config::PbsConfig;
    ///
    /// let config = PbsConfig {
    ///     endpoint: "https://pbs.example.com:8007".to_string(),
    ///     token_id: "user@pam!token".to_string(),
    ///     token_secret: "secret".to_string(),
    ///     verify_tls: false,
    ///     timeout_seconds: 5,
    ///     snapshot_history_limit: 0,
    /// };
    /// let client = PbsClient::new(config).unwrap();
    /// ```
    pub fn new(config: PbsConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .danger_accept_invalid_certs(!config.verify_tls)
            .build()?;

        let auth_header = format!("PBSAPIToken={}:{}", config.token_id, config.token_secret);

        Ok(Self {
            client,
            config,
            auth_header,
        })
    }

    async fn get_api_data<T>(
        &self,
        path: &str,
        request_description: &str,
        parse_error_context: Option<String>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}{}", self.config.endpoint, path);
        debug!("Fetching {} from: {}", request_description, url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .send()
            .await?;

        if !response.status().is_success() {
            warn!(
                "Failed to get {}: {}",
                request_description,
                response.status()
            );
            return Err(PbsError::Api(response.error_for_status().unwrap_err()));
        }

        let api_response: ApiResponse<T> = match parse_error_context {
            Some(context) => response
                .json()
                .await
                .map_err(|e| PbsError::ParseError(format!("{}: {}", context, e)))?,
            None => response.json().await?,
        };

        Ok(api_response.data)
    }

    /// Get node status (CPU, memory, disk, etc.).
    ///
    /// Fetches the current status of the PBS node including CPU usage, memory,
    /// swap, disk usage, load averages, and uptime.
    ///
    /// # Returns
    ///
    /// Returns a `NodeStatus` struct containing all node metrics, or an error if
    /// the API call fails or returns invalid data.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API request fails (network issues, authentication failure)
    /// - The response status is not successful
    /// - The response JSON cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pbs_exporter::client::PbsClient;
    /// # use pbs_exporter::config::PbsConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = PbsConfig {
    /// #     endpoint: "https://pbs.example.com:8007".to_string(),
    /// #     token_id: "user@pam!token".to_string(),
    /// #     token_secret: "secret".to_string(),
    /// #     verify_tls: false,
    /// #     timeout_seconds: 5,
    /// #     snapshot_history_limit: 0,
    /// # };
    /// # let client = PbsClient::new(config)?;
    /// let status = client.get_node_status().await?;
    /// println!("CPU usage: {}", status.cpu);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_node_status(&self) -> Result<NodeStatus> {
        self.get_api_data(
            "/api2/json/nodes/localhost/status",
            "node status",
            Some("Failed to parse node status".to_string()),
        )
        .await
    }

    /// Get datastore usage information.
    ///
    /// Fetches usage statistics for all configured datastores including total,
    /// used, and available bytes.
    ///
    /// # Returns
    ///
    /// Returns a vector of `DatastoreUsage` structs, one for each datastore,
    /// or an error if the API call fails.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or the response cannot be parsed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pbs_exporter::client::PbsClient;
    /// # use pbs_exporter::config::PbsConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = PbsConfig {
    /// #     endpoint: "https://pbs.example.com:8007".to_string(),
    /// #     token_id: "user@pam!token".to_string(),
    /// #     token_secret: "secret".to_string(),
    /// #     verify_tls: false,
    /// #     timeout_seconds: 5,
    /// #     snapshot_history_limit: 0,
    /// # };
    /// # let client = PbsClient::new(config)?;
    /// let datastores = client.get_datastore_usage().await?;
    /// for ds in datastores {
    ///     println!("Datastore {}: {} bytes used", ds.store, ds.used);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_datastore_usage(&self) -> Result<Vec<DatastoreUsage>> {
        self.get_api_data("/api2/json/status/datastore-usage", "datastore usage", None)
            .await
    }

    /// Get backup groups for a specific datastore.
    ///
    /// Fetches all backup groups (VM and container backups) from the specified
    /// datastore including backup count and last backup timestamp.
    ///
    /// # Arguments
    ///
    /// * `datastore` - Name of the datastore to query
    ///
    /// # Returns
    ///
    /// Returns a vector of `BackupGroup` structs containing backup type, ID,
    /// count, and last backup time, or an error if the API call fails.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The datastore doesn't exist or is not accessible
    /// - The API request fails
    /// - The response cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pbs_exporter::client::PbsClient;
    /// # use pbs_exporter::config::PbsConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = PbsConfig {
    /// #     endpoint: "https://pbs.example.com:8007".to_string(),
    /// #     token_id: "user@pam!token".to_string(),
    /// #     token_secret: "secret".to_string(),
    /// #     verify_tls: false,
    /// #     timeout_seconds: 5,
    /// #     snapshot_history_limit: 0,
    /// # };
    /// # let client = PbsClient::new(config)?;
    /// let groups = client.get_backup_groups("backup").await?;
    /// for group in groups {
    ///     println!("{}/{}: {} backups", group.backup_type, group.backup_id, group.backup_count);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_backup_groups(&self, datastore: &str) -> Result<Vec<BackupGroup>> {
        self.get_api_data(
            &format!("/api2/json/admin/datastore/{}/groups", datastore),
            &format!("backup groups for {}", datastore),
            Some(format!("Failed to parse backup groups for {}", datastore)),
        )
        .await
    }

    /// Get PBS version information.
    ///
    /// Fetches the current version, release, and repository ID of the
    /// Proxmox Backup Server.
    ///
    /// # Returns
    ///
    /// Returns a `VersionInfo` struct containing version string, release
    /// information, and repository ID, or an error if the API call fails.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API request fails (network issues, authentication failure)
    /// - The response status is not successful
    /// - The response JSON cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pbs_exporter::client::PbsClient;
    /// # use pbs_exporter::config::PbsConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = PbsConfig {
    /// #     endpoint: "https://pbs.example.com:8007".to_string(),
    /// #     token_id: "user@pam!token".to_string(),
    /// #     token_secret: "secret".to_string(),
    /// #     verify_tls: false,
    /// #     timeout_seconds: 5,
    /// #     snapshot_history_limit: 0,
    /// # };
    /// # let client = PbsClient::new(config)?;
    /// let version = client.get_version().await?;
    /// println!("PBS version: {} ({})", version.version, version.release);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_version(&self) -> Result<VersionInfo> {
        self.get_api_data("/api2/json/version", "version", None)
            .await
    }
}

/// Generic PBS API response wrapper.
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: T,
}

impl PbsClient {
    /// Get snapshots for a specific datastore.
    ///
    /// Fetches all backup snapshots from the specified datastore including
    /// backup time, size, protection status, verification status, and comments.
    /// This is useful for extracting snapshot metadata and comments.
    ///
    /// # Arguments
    ///
    /// * `datastore` - Name of the datastore to query
    ///
    /// # Returns
    ///
    /// Returns a vector of `Snapshot` structs containing snapshot metadata
    /// including backup type, ID, timestamp, size, protection status,
    /// verification status, and optional comments.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The datastore doesn't exist or is not accessible
    /// - The API request fails
    /// - The response JSON cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pbs_exporter::client::PbsClient;
    /// # use pbs_exporter::config::PbsConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = PbsConfig {
    /// #     endpoint: "https://pbs.example.com:8007".to_string(),
    /// #     token_id: "user@pam!token".to_string(),
    /// #     token_secret: "secret".to_string(),
    /// #     verify_tls: false,
    /// #     timeout_seconds: 5,
    /// #     snapshot_history_limit: 0,
    /// # };
    /// # let client = PbsClient::new(config)?;
    /// let snapshots = client.get_snapshots("backup").await?;
    /// for snapshot in snapshots {
    ///     println!("{}/{} at {}", snapshot.backup_type, snapshot.backup_id, snapshot.backup_time);
    ///     if let Some(comment) = snapshot.comment {
    ///         println!("  Comment: {}", comment);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_snapshots(&self, datastore: &str, namespace: &str) -> Result<Vec<Snapshot>> {
        let snapshots: Vec<Snapshot> = self
            .get_api_data(
                &format!("/api2/json/admin/datastore/{}/snapshots?ns={}", datastore, namespace),
                &format!("snapshots for {}", datastore),
                Some(format!("Failed to parse snapshots for {}", datastore)),
            )
            .await?;

        debug!("Fetched {} snapshots for {} in namespace {}", snapshots.len(), datastore, namespace);
        Ok(snapshots)
    }
}


impl PbsClient {
    /// Get namespaces for a specific datastore.
    ///
    /// Fetches all backup namespaces from the specified datastore.
    /// This is needed for extracting snapshot metadata and comments.
    ///
    /// # Arguments
    ///
    /// * `datastore` - Name of the datastore to query
    ///
    /// # Returns
    ///
    /// Returns a vector of `Namespace` structs containing namespace names.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The datastore doesn't exist or is not accessible
    /// - The API request fails
    /// - The response JSON cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pbs_exporter::client::PbsClient;
    /// # use pbs_exporter::config::PbsConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = PbsConfig {
    /// #     endpoint: "https://pbs.example.com:8007".to_string(),
    /// #     token_id: "user@pam!token".to_string(),
    /// #     token_secret: "secret".to_string(),
    /// #     verify_tls: false,
    /// #     timeout_seconds: 5,
    /// #     snapshot_history_limit: 0,
    /// # };
    /// # let client = PbsClient::new(config)?;
    /// let namespaces = client.get_namespaces("backup").await?;
    /// for namespace in namespaces {
    ///     println!("{}", namespace.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_namespaces(&self, datastore: &str) -> Result<Vec<Namespace>> {
        let namespaces: Vec<Namespace> = self
            .get_api_data(
                &format!("/api2/json/admin/datastore/{}/namespace", datastore),
                &format!("namespaces for {}", datastore),
                Some(format!("Failed to parse namespaces for {}", datastore)),
            )
            .await?;

        debug!("Fetched {} namespaces for {}", namespaces.len(), datastore);
        Ok(namespaces)
    }
}

impl PbsClient {
    /// Get recent tasks from PBS.
    ///
    /// Fetches a list of recent tasks (backup, verify, prune, sync, garbage
    /// collection) from the PBS node. Tasks include their start time, end time
    /// (if finished), status, and type.
    ///
    /// # Arguments
    ///
    /// * `limit` - Optional maximum number of tasks to return (default: 50)
    ///
    /// # Returns
    ///
    /// Returns a vector of `Task` structs containing task information including
    /// UPID, worker type, worker ID, start/end times, and status.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API request fails (network issues, authentication failure)
    /// - The response status is not successful
    /// - The response JSON cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pbs_exporter::client::PbsClient;
    /// # use pbs_exporter::config::PbsConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = PbsConfig {
    /// #     endpoint: "https://pbs.example.com:8007".to_string(),
    /// #     token_id: "user@pam!token".to_string(),
    /// #     token_secret: "secret".to_string(),
    /// #     verify_tls: false,
    /// #     timeout_seconds: 5,
    /// #     snapshot_history_limit: 0,
    /// # };
    /// # let client = PbsClient::new(config)?;
    /// let tasks = client.get_tasks(Some(10)).await?;
    /// for task in tasks {
    ///     println!("Task: {} ({})", task.worker_type, task.upid);
    ///     if let Some(status) = task.status {
    ///         println!("  Status: {}", status);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_tasks(&self, limit: Option<u64>) -> Result<Vec<Task>> {
        let limit_param = limit.unwrap_or(50);
        self.get_api_data(
            &format!("/api2/json/nodes/localhost/tasks?limit={}", limit_param),
            "tasks",
            None,
        )
        .await
    }

    /// Get garbage collection status for a datastore.
    ///
    /// Fetches garbage collection (GC) statistics for the specified datastore
    /// including total disk usage, bytes reclaimed, pending bytes that can be
    /// reclaimed, last GC run time, status, and duration.
    ///
    /// # Arguments
    ///
    /// * `datastore` - Name of the datastore to query
    ///
    /// # Returns
    ///
    /// Returns a `GcStatus` struct containing GC statistics including disk bytes,
    /// removed bytes, pending bytes, last run time, state, and duration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The datastore doesn't exist or is not accessible
    /// - The API request fails
    /// - The response JSON cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pbs_exporter::client::PbsClient;
    /// # use pbs_exporter::config::PbsConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = PbsConfig {
    /// #     endpoint: "https://pbs.example.com:8007".to_string(),
    /// #     token_id: "user@pam!token".to_string(),
    /// #     token_secret: "secret".to_string(),
    /// #     verify_tls: false,
    /// #     timeout_seconds: 5,
    /// #     snapshot_history_limit: 0,
    /// # };
    /// # let client = PbsClient::new(config)?;
    /// let gc_status = client.get_gc_status("backup").await?;
    /// if let Some(pending) = gc_status.pending_bytes {
    ///     println!("Pending GC bytes: {}", pending);
    /// }
    /// if let Some(state) = gc_status.last_run_state {
    ///     println!("Last GC state: {}", state);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_gc_status(&self, datastore: &str) -> Result<GcStatus> {
        self.get_api_data(
            &format!("/api2/json/admin/datastore/{}/gc", datastore),
            &format!("GC status for {}", datastore),
            None,
        )
        .await
    }

    /// Get configured tape drives.
    ///
    /// Fetches information about all configured tape drives in the PBS system
    /// including name, vendor, model, and serial number. This is useful for
    /// monitoring tape backup infrastructure.
    ///
    /// # Returns
    ///
    /// Returns a vector of `TapeDrive` structs containing drive information,
    /// or an empty vector if no tape drives are configured.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API request fails (network issues, authentication failure)
    /// - The response status is not successful
    /// - The response JSON cannot be parsed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pbs_exporter::client::PbsClient;
    /// # use pbs_exporter::config::PbsConfig;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = PbsConfig {
    /// #     endpoint: "https://pbs.example.com:8007".to_string(),
    /// #     token_id: "user@pam!token".to_string(),
    /// #     token_secret: "secret".to_string(),
    /// #     verify_tls: false,
    /// #     timeout_seconds: 5,
    /// #     snapshot_history_limit: 0,
    /// # };
    /// # let client = PbsClient::new(config)?;
    /// let drives = client.get_tape_drives().await?;
    /// for drive in drives {
    ///     println!("Drive: {}", drive.name);
    ///     if let Some(model) = drive.model {
    ///         println!("  Model: {}", model);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_tape_drives(&self) -> Result<Vec<TapeDrive>> {
        self.get_api_data("/api2/json/tape/drive", "tape drives", None)
            .await
    }
}
