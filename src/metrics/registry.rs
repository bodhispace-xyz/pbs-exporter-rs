//! Metric registry and builder pattern for reducing repetitive registration code.

use crate::error::{PbsError, Result};
use prometheus::{Encoder, Gauge, GaugeVec, Opts, Registry, TextEncoder};

/// Helper struct for building metrics with less boilerplate.
struct MetricBuilder<'a> {
    registry: &'a Registry,
}

impl<'a> MetricBuilder<'a> {
    fn new(registry: &'a Registry) -> Self {
        Self { registry }
    }

    /// Create and register a Gauge metric.
    fn gauge(&self, name: &str, help: &str) -> Result<Gauge> {
        let gauge = Gauge::with_opts(Opts::new(name, help))
            .map_err(|e| PbsError::Metrics(e.to_string()))?;
        self.registry
            .register(Box::new(gauge.clone()))
            .map_err(|e| PbsError::Metrics(e.to_string()))?;
        Ok(gauge)
    }

    /// Create and register a GaugeVec metric.
    fn gauge_vec(&self, name: &str, help: &str, labels: &[&str]) -> Result<GaugeVec> {
        let gauge_vec = GaugeVec::new(Opts::new(name, help), labels)
            .map_err(|e| PbsError::Metrics(e.to_string()))?;
        self.registry
            .register(Box::new(gauge_vec.clone()))
            .map_err(|e| PbsError::Metrics(e.to_string()))?;
        Ok(gauge_vec)
    }
}

/// Registry holding all metric instances.
#[derive(Clone)]
pub struct MetricRegistry {
    registry: Registry,

    // Exporter metrics
    pub(crate) pbs_up: Gauge,
    pub(crate) exporter_scrape_duration_seconds: Gauge,
    pub(crate) exporter_memory_usage_bytes: Gauge,
    #[allow(dead_code)] // Reserved for future API call tracking
    pub(crate) exporter_api_calls_total: GaugeVec,

    // Host metrics
    pub(crate) host_cpu_usage: Gauge,
    pub(crate) host_io_wait: Gauge,
    pub(crate) host_load1: Gauge,
    pub(crate) host_load5: Gauge,
    pub(crate) host_load15: Gauge,
    pub(crate) host_memory_used_bytes: Gauge,
    pub(crate) host_memory_total_bytes: Gauge,
    pub(crate) host_memory_free_bytes: Gauge,
    pub(crate) host_swap_used_bytes: Gauge,
    pub(crate) host_swap_total_bytes: Gauge,
    pub(crate) host_swap_free_bytes: Gauge,
    pub(crate) host_rootfs_used_bytes: Gauge,
    pub(crate) host_rootfs_total_bytes: Gauge,
    pub(crate) host_rootfs_avail_bytes: Gauge,
    pub(crate) host_uptime_seconds: Gauge,

    // Datastore metrics
    pub(crate) datastore_total_bytes: GaugeVec,
    pub(crate) datastore_used_bytes: GaugeVec,
    pub(crate) datastore_available_bytes: GaugeVec,

    // Backup metrics
    pub(crate) snapshot_count: GaugeVec,
    pub(crate) snapshot_last_timestamp_seconds: GaugeVec,

    // Individual snapshot metrics
    pub(crate) snapshot_info: GaugeVec,
    pub(crate) snapshot_size_bytes: GaugeVec,
    pub(crate) snapshot_verified: GaugeVec,
    pub(crate) snapshot_verification_timestamp: GaugeVec,
    pub(crate) snapshot_protected: GaugeVec,

    // Task metrics
    pub(crate) task_total: GaugeVec,
    pub(crate) task_duration_seconds: GaugeVec,
    pub(crate) task_last_run_timestamp: GaugeVec,
    pub(crate) task_running: GaugeVec,

    // GC metrics
    pub(crate) gc_last_run_timestamp: GaugeVec,
    pub(crate) gc_duration_seconds: GaugeVec,
    pub(crate) gc_removed_bytes: GaugeVec,
    pub(crate) gc_pending_bytes: GaugeVec,
    pub(crate) gc_status: GaugeVec,

    // Tape metrics
    pub(crate) tape_drive_info: GaugeVec,
    pub(crate) tape_drive_available: Gauge,

    // Version info
    pub(crate) pbs_version: GaugeVec,
}

impl MetricRegistry {
    /// Create a new metric registry with all metrics registered.
    ///
    /// This initializes the Prometheus registry and registers all PBS exporter metrics using
    /// the MetricBuilder pattern to reduce boilerplate code.
    ///
    /// # Returns
    ///
    /// Returns a Result containing the initialized MetricRegistry with all metrics registered,
    /// or an error if any metric registration fails.
    ///
    /// # Metrics Registered
    ///
    /// - Exporter metrics (pbs_up, scrape_duration, memory_usage, api_calls)
    /// - Host/node metrics (CPU, memory, swap, disk, load, uptime)
    /// - Datastore metrics (total, used, available bytes)
    /// - Snapshot metrics (count, timestamp, size, verification, protection)
    /// - Task metrics (total, duration, last_run, running)
    /// - Garbage collection metrics (timestamp, duration, removed/pending bytes, status)
    /// - Tape drive metrics (info, available count)
    /// - Version information
    pub fn new() -> Result<Self> {
        let registry = Registry::new();
        let builder = MetricBuilder::new(&registry);

        Ok(Self {
            // Exporter metrics
            pbs_up: builder.gauge(
                "pbs_up",
                "Whether the last scrape of PBS was successful (1 = success, 0 = failure)",
            )?,
            exporter_scrape_duration_seconds: builder.gauge(
                "pbs_exporter_scrape_duration_seconds",
                "Duration of the last scrape in seconds",
            )?,
            exporter_memory_usage_bytes: builder.gauge(
                "pbs_exporter_memory_usage_bytes",
                "Current memory usage of the exporter in bytes",
            )?,
            exporter_api_calls_total: builder.gauge_vec(
                "pbs_exporter_api_calls_total",
                "Total number of API calls made to PBS",
                &["endpoint", "status"],
            )?,

            // Host metrics
            host_cpu_usage: builder.gauge(
                "pbs_host_cpu_usage",
                "CPU usage of the PBS host (fraction of 1.0)",
            )?,
            host_io_wait: builder.gauge(
                "pbs_host_io_wait",
                "CPU I/O wait proportion (fraction of 1.0)",
            )?,
            host_load1: builder.gauge("pbs_host_load1", "1-minute load average")?,
            host_load5: builder.gauge("pbs_host_load5", "5-minute load average")?,
            host_load15: builder.gauge("pbs_host_load15", "15-minute load average")?,
            host_memory_used_bytes: builder.gauge(
                "pbs_host_memory_used_bytes",
                "Used RAM on PBS host in bytes",
            )?,
            host_memory_total_bytes: builder.gauge(
                "pbs_host_memory_total_bytes",
                "Total RAM on PBS host in bytes",
            )?,
            host_memory_free_bytes: builder.gauge(
                "pbs_host_memory_free_bytes",
                "Free RAM on PBS host in bytes",
            )?,
            host_swap_used_bytes: builder
                .gauge("pbs_host_swap_used_bytes", "Used swap space in bytes")?,
            host_swap_total_bytes: builder
                .gauge("pbs_host_swap_total_bytes", "Total swap space in bytes")?,
            host_swap_free_bytes: builder
                .gauge("pbs_host_swap_free_bytes", "Free swap space in bytes")?,
            host_rootfs_used_bytes: builder.gauge(
                "pbs_host_rootfs_used_bytes",
                "Used bytes on root filesystem",
            )?,
            host_rootfs_total_bytes: builder.gauge(
                "pbs_host_rootfs_total_bytes",
                "Total bytes on root filesystem",
            )?,
            host_rootfs_avail_bytes: builder.gauge(
                "pbs_host_rootfs_avail_bytes",
                "Available bytes on root filesystem",
            )?,
            host_uptime_seconds: builder
                .gauge("pbs_host_uptime_seconds", "Uptime of PBS host in seconds")?,

            // Datastore metrics
            datastore_total_bytes: builder.gauge_vec(
                "pbs_datastore_total_bytes",
                "Total size of datastore in bytes",
                &["datastore"],
            )?,
            datastore_used_bytes: builder.gauge_vec(
                "pbs_datastore_used_bytes",
                "Used bytes in datastore",
                &["datastore"],
            )?,
            datastore_available_bytes: builder.gauge_vec(
                "pbs_datastore_available_bytes",
                "Available bytes in datastore",
                &["datastore"],
            )?,

            // Backup metrics
            snapshot_count: builder.gauge_vec(
                "pbs_snapshot_count",
                "Number of backup snapshots",
                &["datastore", "namespace", "backup_type", "backup_id", "comment"],
            )?,
            snapshot_last_timestamp_seconds: builder.gauge_vec(
                "pbs_snapshot_last_timestamp_seconds",
                "Unix timestamp of last backup",
                &["datastore", "namespace", "backup_type", "backup_id", "comment"],
            )?,

            // Individual snapshot metrics
            snapshot_info: builder.gauge_vec(
                "pbs_snapshot_info",
                "Individual snapshot information with timestamp as value",
                &[
                    "datastore",
                    "namespace",
                    "backup_type",
                    "backup_id",
                    "comment",
                    "timestamp",
                ],
            )?,
            snapshot_size_bytes: builder.gauge_vec(
                "pbs_snapshot_size_bytes",
                "Size of individual snapshot in bytes",
                &[
                    "datastore",
                    "namespace",
                    "backup_type",
                    "backup_id",
                    "comment",
                    "timestamp",
                    "verified",
                ],
            )?,
            snapshot_verification_timestamp: builder.gauge_vec(
                "pbs_snapshot_verification_timestamp_seconds",
                "Timestamp of last verification in seconds",
                &[
                    "datastore",
                    "namespace",
                    "backup_type",
                    "backup_id",
                    "comment",
                    "timestamp",
                ],
            )?,
            snapshot_verified: builder.gauge_vec(
                "pbs_snapshot_verified",
                "Snapshot verification status (1=ok, 0=failed/unknown)",
                &[
                    "datastore",
                    "namespace",
                    "backup_type",
                    "backup_id",
                    "comment",
                    "timestamp",
                ],
            )?,
            snapshot_protected: builder.gauge_vec(
                "pbs_snapshot_protected",
                "Snapshot protection status (1=protected, 0=not protected)",
                &[
                    "datastore",
                    "namespace",
                    "backup_type",
                    "backup_id",
                    "comment",
                    "timestamp",
                ],
            )?,

            // Task metrics
            task_total: builder.gauge_vec(
                "pbs_task_total",
                "Total number of tasks (by worker type/status)",
                &["worker_type", "status", "comment"],
            )?,
            task_duration_seconds: builder.gauge_vec(
                "pbs_task_duration_seconds",
                "Task duration in seconds",
                &["worker_type", "status", "worker_id", "comment"],
            )?,
            task_last_run_timestamp: builder.gauge_vec(
                "pbs_task_last_run_timestamp",
                "Last run timestamp for task type",
                &["worker_type"],
            )?,
            task_running: builder.gauge_vec(
                "pbs_task_running",
                "Currently running tasks",
                &["worker_type", "comment"],
            )?,

            // GC metrics
            gc_last_run_timestamp: builder.gauge_vec(
                "pbs_gc_last_run_timestamp",
                "Last GC run completion timestamp",
                &["datastore"],
            )?,
            gc_duration_seconds: builder.gauge_vec(
                "pbs_gc_duration_seconds",
                "Last GC duration in seconds",
                &["datastore"],
            )?,
            gc_removed_bytes: builder.gauge_vec(
                "pbs_gc_removed_bytes",
                "Bytes reclaimed in last GC",
                &["datastore"],
            )?,
            gc_pending_bytes: builder.gauge_vec(
                "pbs_gc_pending_bytes",
                "Bytes that can be reclaimed by GC",
                &["datastore"],
            )?,
            gc_status: builder.gauge_vec(
                "pbs_gc_status",
                "Last GC status (1=OK, 0=ERROR)",
                &["datastore"],
            )?,

            // Tape metrics
            tape_drive_info: builder.gauge_vec(
                "pbs_tape_drive_info",
                "Tape drive information",
                &["name", "vendor", "model", "serial"],
            )?,
            tape_drive_available: builder.gauge(
                "pbs_tape_drive_available",
                "Number of available tape drives",
            )?,

            // Version info
            pbs_version: builder.gauge_vec(
                "pbs_version",
                "PBS version information",
                &["version", "release", "repoid"],
            )?,

            registry,
        })
    }

    /// Encode metrics in Prometheus text format.
    ///
    /// Serializes all metrics in the registry to Prometheus exposition format.
    /// Uses a thread-local buffer to minimize allocations on repeated calls.
    ///
    /// # Returns
    ///
    /// Returns a Result containing the encoded metrics as a UTF-8 String,
    /// or an error if encoding fails.
    pub fn encode(&self) -> Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();

        // Keep a thread-local scratch buffer while encoding the current scrape.
        thread_local! {
            static BUFFER: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(Vec::with_capacity(8192));
        }

        BUFFER.with(|buf| {
            let mut buffer = buf.borrow_mut();
            buffer.clear();

            encoder
                .encode(&metric_families, &mut *buffer)
                .map_err(|e| PbsError::Metrics(e.to_string()))?;

            // Move bytes out instead of cloning to save memory
            let bytes = std::mem::take(&mut *buffer);
            String::from_utf8(bytes).map_err(|e| PbsError::Metrics(e.to_string()))
        })
    }
}
