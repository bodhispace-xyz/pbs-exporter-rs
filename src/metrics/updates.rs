//! Metric update functions.
//!
//! This module contains all the functions that update metrics based on PBS API data.

use super::MetricRegistry;
use crate::client::{
    BackupGroup, DatastoreUsage, GcStatus, NodeStatus, Snapshot, TapeDrive, Task,
    VerificationStatus, VersionInfo,
};
use std::collections::HashMap;
use tracing::debug;

// Interned strings to avoid repeated allocations
const UNKNOWN: &str = "unknown";
const EMPTY_STR: &str = "";
const RUNNING: &str = "running";
const OK: &str = "ok";

type LatestSnapshotCommentMap = HashMap<(String, String), (i64, Option<String>)>;

struct SnapshotVerification {
    value: f64,
    label: &'static str,
    timestamp: Option<i64>,
}

pub(super) fn update_node_metrics(metrics: &MetricRegistry, status: &NodeStatus) {
    debug!("Updating node metrics");
    metrics.host_cpu_usage.set(status.cpu);
    metrics.host_io_wait.set(status.wait);
    metrics.host_load1.set(status.loadavg[0]);
    metrics.host_load5.set(status.loadavg[1]);
    metrics.host_load15.set(status.loadavg[2]);
    metrics
        .host_memory_used_bytes
        .set(status.memory.used as f64);
    metrics
        .host_memory_total_bytes
        .set(status.memory.total as f64);
    metrics
        .host_memory_free_bytes
        .set(status.memory.free as f64);
    metrics.host_swap_used_bytes.set(status.swap.used as f64);
    metrics.host_swap_total_bytes.set(status.swap.total as f64);
    metrics.host_swap_free_bytes.set(status.swap.free as f64);
    metrics.host_rootfs_used_bytes.set(status.root.used as f64);
    metrics
        .host_rootfs_total_bytes
        .set(status.root.total as f64);
    metrics
        .host_rootfs_avail_bytes
        .set(status.root.avail as f64);
    metrics.host_uptime_seconds.set(status.uptime as f64);
}

pub(super) fn update_datastore_metrics(metrics: &MetricRegistry, datastores: &[DatastoreUsage]) {
    debug!(
        "Updating datastore metrics for {} datastores",
        datastores.len()
    );
    for ds in datastores {
        metrics
            .datastore_total_bytes
            .with_label_values(&[&ds.store])
            .set(ds.total as f64);
        metrics
            .datastore_used_bytes
            .with_label_values(&[&ds.store])
            .set(ds.used as f64);
        metrics
            .datastore_available_bytes
            .with_label_values(&[&ds.store])
            .set(ds.avail as f64);
    }
}

pub(super) fn update_snapshot_metrics(
    metrics: &MetricRegistry,
    datastore: &str,
    snapshots: &[Snapshot],
    comment_map: &LatestSnapshotCommentMap,
    snapshot_history_limit: usize,
) {
    debug!(
        "Updating individual snapshot metrics for {} snapshots in {}",
        snapshots.len(),
        datastore
    );

    // Efficient filtering: Sort by Group (Type, ID) then Time Descending
    let mut sorted_snapshots: Vec<_> = snapshots.iter().collect();
    sorted_snapshots.sort_by(|a, b| {
        a.backup_type
            .cmp(&b.backup_type)
            .then_with(|| a.backup_id.cmp(&b.backup_id))
            .then_with(|| b.backup_time.cmp(&a.backup_time)) // Descending time
    });

    let mut exposed_count = 0;
    let mut current_group = None;
    let mut group_counter = 0;

    for snapshot in sorted_snapshots {
        let group_key = (&snapshot.backup_type, &snapshot.backup_id);

        if Some(group_key) != current_group {
            current_group = Some(group_key);
            group_counter = 0;
        }

        if snapshot_history_limit > 0 && group_counter >= snapshot_history_limit {
            continue;
        }
        group_counter += 1;
        exposed_count += 1;

        let size = snapshot.size.unwrap_or(0) as i64;
        let comment = latest_snapshot_comment(snapshot, comment_map);
        let safe_comment = truncate_snapshot_comment(comment);

        let timestamp_seconds = snapshot.backup_time;
        let timestamp_str = timestamp_seconds.to_string();

        // Base labels for most metrics
        let base_labels = [
            datastore,
            &snapshot.backup_type,
            &snapshot.backup_id,
            safe_comment,
            &timestamp_str,
        ];

        // Info metric (timestamp)
        metrics
            .snapshot_info
            .with_label_values(&base_labels)
            .set(timestamp_seconds as f64);

        let verification = snapshot_verification(snapshot.verification.as_ref());

        // Size metric needs extra "verified" label
        let size_labels = [
            datastore,
            &snapshot.backup_type,
            &snapshot.backup_id,
            safe_comment,
            &timestamp_str,
            verification.label,
        ];

        metrics
            .snapshot_size_bytes
            .with_label_values(&size_labels)
            .set(size as f64);

        // Verification timestamp metric
        if let Some(ts) = verification.timestamp {
            metrics
                .snapshot_verification_timestamp
                .with_label_values(&base_labels)
                .set(ts as f64);
        }

        metrics
            .snapshot_verified
            .with_label_values(&base_labels)
            .set(verification.value);

        // Protection status
        let protected = if snapshot.protected.unwrap_or(false) {
            1.0
        } else {
            0.0
        };
        metrics
            .snapshot_protected
            .with_label_values(&base_labels)
            .set(protected);
    }

    debug!(
        "Exposed {}/{} snapshots for datastore {} (limit: {})",
        exposed_count,
        snapshots.len(),
        datastore,
        snapshot_history_limit
    );
}

fn latest_snapshot_comment<'a>(
    snapshot: &Snapshot,
    comment_map: &'a LatestSnapshotCommentMap,
) -> &'a str {
    // Optimize: clone the key only once for lookup.
    let lookup_key = (snapshot.backup_type.clone(), snapshot.backup_id.clone());

    comment_map
        .get(&lookup_key)
        .and_then(|(_time, comment)| comment.as_deref())
        .unwrap_or(EMPTY_STR)
}

fn truncate_snapshot_comment(comment: &str) -> &str {
    if comment.len() > 50 {
        &comment[..47]
    } else {
        comment
    }
}

fn truncate_backup_comment(comment: &str) -> &str {
    if comment.len() > 50 {
        &comment[..50]
    } else {
        comment
    }
}

fn snapshot_verification(verification: Option<&VerificationStatus>) -> SnapshotVerification {
    if let Some(ver) = verification {
        let is_ok = ver.state == OK;
        SnapshotVerification {
            value: if is_ok { 1.0 } else { 0.0 },
            label: if is_ok { "true" } else { "false" },
            timestamp: ver.last_verify,
        }
    } else {
        SnapshotVerification {
            value: 0.0,
            label: "false",
            timestamp: None,
        }
    }
}

pub(super) fn update_backup_metrics(
    metrics: &MetricRegistry,
    datastore: &str,
    groups: &[BackupGroup],
    comment_map: &LatestSnapshotCommentMap,
) {
    debug!(
        "Updating backup metrics for {} groups in {}",
        groups.len(),
        datastore
    );
    for group in groups {
        // Get comment from the latest snapshot via comment_map
        // Optimize: Clone key only once for lookup
        let lookup_key = (group.backup_type.clone(), group.backup_id.clone());
        let comment = comment_map
            .get(&lookup_key)
            .and_then(|(_time, comment)| comment.as_deref())
            .unwrap_or(EMPTY_STR);
        let truncated_comment = truncate_backup_comment(comment);

        let labels = &[
            datastore,
            &group.backup_type,
            &group.backup_id,
            truncated_comment,
        ];

        metrics
            .snapshot_count
            .with_label_values(labels)
            .set(group.backup_count as f64);

        metrics
            .snapshot_last_timestamp_seconds
            .with_label_values(labels)
            .set(group.last_backup as f64);
    }
}

pub(super) fn update_task_metrics(
    metrics: &MetricRegistry,
    tasks: &[Task],
    comment_map: &HashMap<String, String>,
) {
    debug!("Updating task metrics for {} tasks", tasks.len());

    // Count tasks by type and status and comment - use &str to avoid clones
    // Pre-allocate with estimated capacity
    let mut task_counts: HashMap<(&str, &str, &str), u64> = HashMap::with_capacity(tasks.len());
    // Count running tasks by type and comment
    let mut running_counts: HashMap<(&str, &str), u64> = HashMap::with_capacity(tasks.len() / 2);

    // Track latest finished task timestamp per worker type
    let mut last_run_map: HashMap<&str, i64> = HashMap::new();

    for task in tasks {
        // Use as_deref to avoid cloning
        let comment = task.comment.as_deref().unwrap_or_else(|| {
            // If comment is empty, try to look it up in the map (from snapshots)
            task.worker_id
                .as_ref()
                .and_then(|wid| comment_map.get(wid.as_str()))
                .map(|s| s.as_str())
                .unwrap_or(EMPTY_STR)
        });

        // Unwrap status or use "unknown"
        let status = task.status.as_deref().unwrap_or(UNKNOWN);

        // Count by type and status and comment - clone only when inserting
        let key = (task.worker_type.as_str(), status, comment);
        *task_counts.entry(key).or_insert(0) += 1;

        // Track currently running tasks
        // If endtime is None, it's running. status might be "running" or something else.
        // pbs_task_running only tracks worker_type and comment
        if task.endtime.is_none() || status == RUNNING {
            let run_key = (task.worker_type.as_str(), comment);
            *running_counts.entry(run_key).or_insert(0) += 1;
        } else if let Some(endtime) = task.endtime {
            // Calculate duration for finished tasks
            let duration = endtime - task.starttime;
            // Use empty string for worker_id if None
            let worker_id = task.worker_id.as_deref().unwrap_or(UNKNOWN);

            metrics
                .task_duration_seconds
                .with_label_values(&[task.worker_type.as_str(), status, worker_id, comment])
                .set(duration as f64);

            // Record the latest finished task timestamp for this worker type
            let entry = last_run_map.entry(task.worker_type.as_str()).or_insert(endtime);
            if endtime > *entry {
                *entry = endtime;
            }
        }
    }

    // Set last run timestamp for each worker type to the most recent endtime
    for (worker_type, latest_endtime) in last_run_map {
        metrics
            .task_last_run_timestamp
            .with_label_values(&[worker_type])
            .set(latest_endtime as f64);
    }

    // Update total task counts
    for ((worker_type, status, comment), count) in task_counts {
        metrics
            .task_total
            .with_label_values(&[worker_type, status, comment])
            .set(count as f64);
    }

    // Update running task counts
    for ((worker_type, comment), count) in running_counts {
        metrics
            .task_running
            .with_label_values(&[worker_type, comment])
            .set(count as f64);
    }
}

pub(super) fn update_gc_metrics(metrics: &MetricRegistry, datastore: &str, gc_status: &GcStatus) {
    debug!("Updating GC metrics for {}", datastore);

    if let Some(timestamp) = gc_status.last_run_endtime {
        metrics
            .gc_last_run_timestamp
            .with_label_values(&[datastore])
            .set(timestamp as f64);
    }

    if let Some(duration) = gc_status.duration {
        metrics
            .gc_duration_seconds
            .with_label_values(&[datastore])
            .set(duration);
    }

    if let Some(removed) = gc_status.removed_bytes {
        metrics
            .gc_removed_bytes
            .with_label_values(&[datastore])
            .set(removed as f64);
    }

    if let Some(pending) = gc_status.pending_bytes {
        metrics
            .gc_pending_bytes
            .with_label_values(&[datastore])
            .set(pending as f64);
    }

    if let Some(state) = &gc_status.last_run_state {
        let status_value = if state.eq_ignore_ascii_case(OK) {
            1.0
        } else {
            0.0
        };
        metrics
            .gc_status
            .with_label_values(&[datastore])
            .set(status_value);
    }
}

pub(super) fn update_tape_metrics(metrics: &MetricRegistry, drives: &[TapeDrive]) {
    debug!("Updating tape metrics for {} drives", drives.len());

    metrics.tape_drive_available.set(drives.len() as f64);

    for drive in drives {
        let vendor = drive.vendor.as_deref().unwrap_or(UNKNOWN);
        let model = drive.model.as_deref().unwrap_or(UNKNOWN);
        let serial = drive.serial.as_deref().unwrap_or(UNKNOWN);

        metrics
            .tape_drive_info
            .with_label_values(&[drive.name.as_str(), vendor, model, serial])
            .set(1.0);
    }
}

pub(super) fn update_version_metrics(metrics: &MetricRegistry, version: &VersionInfo) {
    debug!("Updating version metrics: {}", version.version);
    metrics
        .pbs_version
        .with_label_values(&[&version.version, &version.release, &version.repoid])
        .set(1.0);
}
