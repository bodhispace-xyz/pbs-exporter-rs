# PBS Exporter

[![CI](https://github.com/yourusername/pbs-exporter-rs/workflows/CI/badge.svg)](https://github.com/yourusername/pbs-exporter-rs/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

A Prometheus metrics exporter for Proxmox Backup Server 4.x, written in Rust.

## Features

- **Comprehensive Metrics**: Host system, datastore, snapshots, tasks, GC, and tape metrics
- **Secure**: API token authentication with TLS support
- **Fast**: Built with Rust and async I/O (Tokio)
- **Prometheus Native**: Standard Prometheus exposition format
- **Docker Ready**: Optimized Alpine-based image (23.4MB)
- **Production Ready**: Comprehensive error handling, logging, and testing
- **Cardinality Control**: Configurable snapshot history limits

## Metrics Exposed

### Host System Metrics

- `pbs_host_cpu_usage` - CPU usage (0.0-1.0)
- `pbs_host_io_wait` - I/O wait time
- `pbs_host_load{1,5,15}` - Load averages
- `pbs_host_memory_{used,total,free}_bytes` - Memory usage
- `pbs_host_swap_{used,total,free}_bytes` - Swap usage
- `pbs_host_rootfs_{used,total,avail}_bytes` - Root filesystem usage
- `pbs_host_uptime_seconds` - System uptime

### Datastore Metrics

- `pbs_datastore_total_bytes{datastore}` - Total datastore size
- `pbs_datastore_used_bytes{datastore}` - Used space
- `pbs_datastore_available_bytes{datastore}` - Available space

### Snapshot Metrics

- `pbs_snapshot_count{datastore,backup_type,backup_id,comment}` - Number of snapshots per group
- `pbs_snapshot_last_timestamp_seconds{datastore,backup_type,backup_id,comment}` - Unix timestamp of last backup per group
- `pbs_snapshot_info{datastore,backup_type,backup_id,comment,timestamp}` - Snapshot metadata with timestamp as value
- `pbs_snapshot_size_bytes{datastore,backup_type,backup_id,comment,timestamp,verified}` - Snapshot size
- `pbs_snapshot_verification_timestamp_seconds{datastore,backup_type,backup_id,comment,timestamp}` - Unix timestamp of last verification
- `pbs_snapshot_verified{datastore,backup_type,backup_id,comment,timestamp}` - Snapshot verification status (1=ok, 0=failed/unknown)
- `pbs_snapshot_protected{datastore,backup_type,backup_id,comment,timestamp}` - Snapshot protection status

### Task Metrics

- `pbs_task_total{worker_type,status,comment}` - Total tasks by type and status
- `pbs_task_duration_seconds{worker_type,status,worker_id,comment}` - Task duration
- `pbs_task_last_run_timestamp{worker_type}` - Last run timestamp for task type
- `pbs_task_running{worker_type,comment}` - Currently running tasks

### Garbage Collection Metrics

- `pbs_gc_last_run_timestamp{datastore}` - Last GC completion time
- `pbs_gc_duration_seconds{datastore}` - Last GC duration
- `pbs_gc_removed_bytes{datastore}` - Bytes reclaimed in last GC
- `pbs_gc_pending_bytes{datastore}` - Bytes that can be reclaimed
- `pbs_gc_status{datastore}` - Last GC status (1=OK, 0=ERROR)

### Tape Metrics

- `pbs_tape_drive_info{name,vendor,model,serial}` - Tape drive information
- `pbs_tape_drive_available` - Number of available tape drives

### Exporter Self-Monitoring Metrics

- `pbs_up` - Exporter health (1 = success, 0 = failure)
- `pbs_version{version,release,repoid}` - PBS version info
- `pbs_exporter_scrape_duration_seconds` - Duration of last scrape operation
- `pbs_exporter_memory_usage_bytes` - Current memory usage of the exporter

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/pbs-exporter-rs.git
cd pbs-exporter-rs

# Build release binary
cargo build --release

# Binary will be at target/release/pbs-exporter
```

### Using Docker

**Optimized Alpine-based image (23.4MB)**:

```bash
# Build optimized image
docker build -t pbs-exporter .

# Run with environment variables
docker run -p 9101:9101 \
  -e PBS_EXPORTER__PBS__ENDPOINT=https://pbs.example.com:8007 \
  -e PBS_EXPORTER__PBS__TOKEN_ID=exporter@pam!metrics \
  -e PBS_EXPORTER__PBS__TOKEN_SECRET=your-secret \
  -e PBS_EXPORTER__PBS__VERIFY_TLS=true \
  pbs-exporter
```

> **Note**: Environment variables use double underscores (`__`) to separate nested configuration fields.

## Configuration

### PBS API Token Setup

1. Log into your PBS web interface
2. Navigate to **Configuration** → **Access Control** → **API Tokens**
3. Create a new token with the following permissions:
   - `Datastore.Audit` on each datastore
   - `Sys.Audit` on the system

### Configuration Methods

The exporter supports three configuration methods (in order of precedence):

1. **Environment Variables** (highest priority)
2. **Configuration File**
3. **Default Values** (lowest priority)

#### Using a Configuration File

Create `config/default.toml`:

```toml
[pbs]
endpoint = "https://pbs.example.com:8007"
token_id = "exporter@pam!metrics"
token_secret = "your-secret-token"
verify_tls = true
timeout_seconds = 5
snapshot_history_limit = 0  # 0=unlimited, 1=latest only, 2=two most recent, etc.

[exporter]
listen_address = "0.0.0.0:9101"
log_level = "info"
```

Run with:

```bash
pbs-exporter --config config/default.toml
```

#### Using Environment Variables

**Important**: Use double underscores (`__`) to separate nested configuration:

```bash
export PBS_EXPORTER__PBS__ENDPOINT=https://pbs.example.com:8007
export PBS_EXPORTER__PBS__TOKEN_ID=exporter@pam!metrics
export PBS_EXPORTER__PBS__TOKEN_SECRET=your-secret
export PBS_EXPORTER__PBS__VERIFY_TLS=true
export PBS_EXPORTER__PBS__SNAPSHOT_HISTORY_LIMIT=0

pbs-exporter
```

**Available Configuration Options**:

| Variable | Default | Description |
|----------|---------|-------------|
| `PBS_EXPORTER__PBS__ENDPOINT` | - | PBS server URL (required) |
| `PBS_EXPORTER__PBS__TOKEN_ID` | - | API token ID (required) |
| `PBS_EXPORTER__PBS__TOKEN_SECRET` | - | API token secret (required) |
| `PBS_EXPORTER__PBS__VERIFY_TLS` | `true` | Verify the PBS certificate and hostname; requires HTTPS |
| `PBS_EXPORTER__PBS__CA_CERT_PATH` | unset | Path to an additional trusted PEM CA/certificate bundle |
| `PBS_EXPORTER__PBS__TIMEOUT_SECONDS` | `5` | API request timeout |
| `PBS_EXPORTER__PBS__SNAPSHOT_HISTORY_LIMIT` | `0` | Max snapshots per group (0=unlimited) |
| `PBS_EXPORTER__EXPORTER__LISTEN_ADDRESS` | `0.0.0.0:9101` | Listen address |
| `PBS_EXPORTER__EXPORTER__LOG_LEVEL` | `info` | Log level (debug/info/warn/error) |

## Usage

### Running the Exporter

```bash
# With config file
pbs-exporter --config config/default.toml

# With environment variables only
PBS_EXPORTER_PBS_ENDPOINT=https://pbs.local:8007 \
PBS_EXPORTER_PBS_TOKEN_ID=token@pam!id \
PBS_EXPORTER_PBS_TOKEN_SECRET=secret \
pbs-exporter
```

### Prometheus Configuration

Add to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'pbs'
    static_configs:
      - targets: ['localhost:9101']
    scrape_interval: 60s
```

### Endpoints

- `http://localhost:9101/metrics` - Prometheus metrics
- `http://localhost:9101/health` - Health check
- `http://localhost:9101/` - Info page

## Development

### Prerequisites

- Rust 1.71 or later
- Cargo

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Linting

```bash
cargo clippy -- -D warnings
cargo fmt --check
```

### Running Locally

```bash
# Copy example config
cp config/default.toml config/local.toml

# Edit with your PBS credentials
vim config/local.toml

# Run in debug mode
RUST_LOG=debug cargo run -- --config config/local.toml
```

## Docker Deployment

### Image Size

The Docker image uses an optimized Alpine Linux base:

- **Size**: 23.4MB (85% smaller than Debian-based images)
- **Base**: `alpine:3.19` with static linking
- **Security**: Runs as non-root user

### Build

```bash
docker build -t pbs-exporter .
```

### Run with Environment Variables

```bash
docker run -d \
  --name pbs-exporter \
  -p 9101:9101 \
  -e PBS_EXPORTER__PBS__ENDPOINT=https://pbs.example.com:8007 \
  -e PBS_EXPORTER__PBS__TOKEN_ID=exporter@pam!metrics \
  -e PBS_EXPORTER__PBS__TOKEN_SECRET=your-secret \
  -e PBS_EXPORTER__PBS__VERIFY_TLS=true \
  --restart unless-stopped \
  pbs-exporter
```

### Docker Compose

Create a `.env` file:

```bash
# PBS Server Configuration
PBS_EXPORTER__PBS__ENDPOINT=https://pbs.example.com:8007
PBS_EXPORTER__PBS__TOKEN_ID=exporter@pam!metrics
PBS_EXPORTER__PBS__TOKEN_SECRET=your-secret-token
PBS_EXPORTER__PBS__VERIFY_TLS=true
PBS_EXPORTER__PBS__TIMEOUT_SECONDS=5

# Snapshot History Limit
# 0 = unlimited, 1 = latest only, 2 = two most recent, etc.
PBS_EXPORTER__PBS__SNAPSHOT_HISTORY_LIMIT=0

# Logging
PBS_EXPORTER__EXPORTER__LOG_LEVEL=info
RUST_LOG=info
```

Then use `docker-compose.yml`:

```yaml
services:
  pbs-exporter:
    image: ghcr.io/mlgruby/pbs-exporter-rs:main
    container_name: pbs-exporter
    ports:
      - "9101:9101"
    env_file:
      - .env
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "--quiet", "--tries=1", "--spider", "http://localhost:9101/health"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 5s
```

### Multi-LXC Deployment

For deploying across multiple LXC containers (e.g., exporter in LXC1, Prometheus in LXC2):

1. **LXC1** (PBS Exporter):

   ```bash
   docker-compose up -d
   # Exporter listens on 0.0.0.0:9101 (accessible from other containers)
   ```

2. **LXC2** (Prometheus):

   ```yaml
   # prometheus.yml
   scrape_configs:
     - job_name: 'pbs'
       static_configs:
         - targets: ['<LXC1_IP>:9101']  # e.g., '192.168.1.100:9101'
       scrape_interval: 60s
   ```

## Troubleshooting

### TLS Certificate Errors

Certificate verification is enabled by default. The PBS endpoint must use HTTPS
when verification is enabled, including redirect destinations. Existing configurations that explicitly set
`verify_tls = false` remain an insecure opt-out and emit a warning.

For a private CA or a self-signed PBS certificate, obtain the PEM CA bundle (or
self-signed server certificate) through a trusted administration channel and verify
its fingerprint. Keep verification enabled and configure its path:

```toml
[pbs]
verify_tls = true
ca_cert_path = "/etc/pbs-exporter/pbs-ca.pem"
```

Or via environment:

```bash
export PBS_EXPORTER__PBS__VERIFY_TLS=true
export PBS_EXPORTER__PBS__CA_CERT_PATH=/etc/pbs-exporter/pbs-ca.pem
```

For Docker Compose, mount the public certificate read-only and use the container
path in the configuration:

```yaml
volumes:
  - ./pbs-ca.pem:/etc/pbs-exporter/pbs-ca.pem:ro
environment:
  PBS_EXPORTER__PBS__CA_CERT_PATH: /etc/pbs-exporter/pbs-ca.pem
```

The URL hostname must match the certificate's subject alternative names. Trusting
a CA does not disable hostname checks. Missing, invalid or empty CA bundles fail
startup; certificate failures do not fall back to insecure TLS.

For temporary testing on a trusted network only, explicitly set
`PBS_EXPORTER__PBS__VERIFY_TLS=false`. This also permits a plain HTTP endpoint and
removes peer authentication. Do not configure a CA bundle with this opt-out: the
exporter rejects that combination so the trust setting cannot be silently ignored.

**Upgrade note:** deployments that previously omitted `verify_tls` and relied on
an untrusted/self-signed certificate must configure their CA before upgrading.
Rust callers constructing `PbsConfig` directly must also provide `ca_cert_path`
(`None` for the normal public trust store).

### Authentication Errors

Ensure your API token has the correct permissions:

- `Datastore.Audit` for datastore and snapshot metrics
- `Sys.Audit` for system and task metrics
- `Tape.Audit` for tape drive metrics (if using tape backups)

### Connection Timeouts

Increase the timeout:

```toml
[pbs]
timeout_seconds = 10
```

Or via environment:

```bash
export PBS_EXPORTER__PBS__TIMEOUT_SECONDS=10
```

### High Metric Cardinality

If you have many snapshots and want to reduce metric cardinality:

```toml
[pbs]
snapshot_history_limit = 1  # Only expose the latest snapshot per backup group
```

Or via environment:

```bash
export PBS_EXPORTER__PBS__SNAPSHOT_HISTORY_LIMIT=1
```

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details on:

- Fork-based workflow
- Branch naming conventions
- Pull request process
- Code style and testing requirements

**Quick Start for Contributors:**

1. Fork the repository
2. Create a feature branch from `develop`
3. Make your changes
4. Submit a PR to `develop` (not `main`)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Acknowledgments

- Built with [Tokio](https://tokio.rs/), [Axum](https://github.com/tokio-rs/axum), and [Prometheus Rust client](https://github.com/tikv/rust-prometheus)
- Inspired by the PBS API and Prometheus ecosystem

## Related Projects

- [Proxmox Backup Server](https://www.proxmox.com/en/proxmox-backup-server)
- [Prometheus](https://prometheus.io/)
