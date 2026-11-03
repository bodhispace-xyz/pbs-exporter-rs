//! PBS API response types used by the client and metrics collector.

use serde::{Deserialize, Serialize};

/// Node status information from PBS.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeStatus {
    /// CPU usage (0.0 to 1.0)
    pub cpu: f64,
    /// I/O wait (0.0 to 1.0)
    pub wait: f64,
    /// Used memory in bytes
    pub memory: Memory,
    /// Root filesystem usage (PBS calls it "root" not "rootfs")
    pub root: Disk,
    /// Swap usage
    pub swap: Memory,
    /// Load averages [1min, 5min, 15min]
    pub loadavg: [f64; 3],
    /// Uptime in seconds
    pub uptime: u64,
}

/// Memory information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Memory {
    /// Used memory in bytes
    pub used: u64,
    /// Total memory in bytes
    pub total: u64,
    /// Free memory in bytes
    pub free: u64,
}

/// Disk information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Disk {
    /// Used disk space in bytes
    pub used: u64,
    /// Total disk space in bytes
    pub total: u64,
    /// Available disk space in bytes
    pub avail: u64,
}

/// Datastore usage information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatastoreUsage {
    /// Datastore name
    pub store: String,
    /// Total size in bytes
    pub total: u64,
    /// Used bytes
    pub used: u64,
    /// Available bytes
    pub avail: u64,
}

/// Backup group information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupGroup {
    /// Backup type (vm, ct, host)
    #[serde(rename = "backup-type")]
    pub backup_type: String,
    /// Backup ID (VM ID, CT ID, or hostname)
    #[serde(rename = "backup-id")]
    pub backup_id: String,
    /// Number of snapshots in this group
    #[serde(rename = "backup-count")]
    pub backup_count: u64,
    /// Last backup timestamp (Unix epoch)
    #[serde(rename = "last-backup")]
    pub last_backup: i64,
    /// Optional comment
    #[serde(default)]
    pub comment: Option<String>,
}

/// Snapshot information from PBS.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Snapshot {
    /// Backup type (vm, ct, host)
    #[serde(rename = "backup-type")]
    pub backup_type: String,
    /// Backup ID (VM ID, CT ID, or hostname)
    #[serde(rename = "backup-id")]
    pub backup_id: String,
    /// Backup timestamp (Unix epoch)
    #[serde(rename = "backup-time")]
    pub backup_time: i64,
    /// Optional comment
    #[serde(default)]
    pub comment: Option<String>,
    /// Total snapshot size in bytes
    #[serde(default)]
    pub size: Option<u64>,
    /// Whether snapshot is protected from deletion
    #[serde(default)]
    pub protected: Option<bool>,
    /// Verification status
    #[serde(default)]
    pub verification: Option<VerificationStatus>,
}

/// Snapshot information from PBS.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Namespace {
    /// Namespace string
    #[serde(rename = "ns")]
    pub name: String,
}

/// Verification status information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VerificationStatus {
    /// Verification state (ok, failed, none, etc.)
    pub state: String,
    /// Last verification timestamp (Unix epoch)
    #[serde(rename = "last-verify")]
    pub last_verify: Option<i64>,
}

/// PBS version information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionInfo {
    /// PBS version string
    pub version: String,
    /// Release information
    pub release: String,
    /// Repository ID
    pub repoid: String,
}

/// Task information from PBS.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Task {
    /// Unique process ID
    pub upid: String,
    /// Worker type (backup, verify, prune, sync, garbage_collection)
    #[serde(rename = "worker_type")]
    pub worker_type: String,
    /// Worker ID (datastore:type/id)
    #[serde(rename = "worker_id")]
    pub worker_id: Option<String>,
    /// Start timestamp
    pub starttime: i64,
    /// Task end time (if finished)
    pub endtime: Option<i64>,
    /// Task status
    pub status: Option<String>,
    /// Comment (if any)
    #[serde(default)]
    pub comment: Option<String>,
}

/// Garbage collection status for a datastore.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GcStatus {
    /// Total bytes on disk
    #[serde(rename = "disk-bytes")]
    pub disk_bytes: Option<u64>,
    /// Bytes reclaimed in last GC
    #[serde(rename = "removed-bytes")]
    pub removed_bytes: Option<u64>,
    /// Bytes that can be reclaimed
    #[serde(rename = "pending-bytes")]
    pub pending_bytes: Option<u64>,
    /// Last GC completion timestamp
    #[serde(rename = "last-run-endtime")]
    pub last_run_endtime: Option<i64>,
    /// Last GC status
    #[serde(rename = "last-run-state")]
    pub last_run_state: Option<String>,
    /// Last GC duration in seconds
    pub duration: Option<f64>,
}

/// Tape drive information.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TapeDrive {
    /// Drive name
    pub name: String,
    /// Vendor
    #[serde(default)]
    pub vendor: Option<String>,
    /// Model
    #[serde(default)]
    pub model: Option<String>,
    /// Serial number
    #[serde(default)]
    pub serial: Option<String>,
}
