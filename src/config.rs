//! Configuration management for PBS exporter.
//!
//! Supports loading configuration from:
//! - TOML configuration files
//! - Environment variables (with `PBS_EXPORTER_` prefix)
//! - Command-line arguments

use crate::error::{PbsError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// PBS server connection settings.
#[derive(Clone, Serialize, Deserialize)]
pub struct PbsConfig {
    /// PBS API endpoint URL (e.g., "<https://pbs.example.com:8007>")
    pub endpoint: String,

    /// API token ID (e.g., "user@pam!tokenid")
    #[serde(default)]
    pub token_id: String,

    /// API token secret
    #[serde(default)]
    pub token_secret: String,

    /// Verify TLS certificates. Prefer a CA bundle for private/self-signed certificates.
    #[serde(default = "default_verify_tls")]
    pub verify_tls: bool,

    /// Optional PEM CA certificate bundle for the PBS endpoint.
    #[serde(default)]
    pub ca_cert_path: Option<String>,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Number of snapshots to expose per backup group (0 = all, 1 = latest only, 2 = 2 latest, etc.)
    #[serde(default = "default_snapshot_history_limit")]
    pub snapshot_history_limit: usize,
}

impl std::fmt::Debug for PbsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PbsConfig")
            .field("endpoint", &self.endpoint)
            .field("token_id", &self.token_id)
            .field("token_secret", &"***REDACTED***")
            .field("verify_tls", &self.verify_tls)
            .field("ca_cert_path", &self.ca_cert_path)
            .field("timeout_seconds", &self.timeout_seconds)
            .field("snapshot_history_limit", &self.snapshot_history_limit)
            .finish()
    }
}

/// Exporter specific settings.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExporterConfig {
    /// Address to listen on for metrics endpoint
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

/// Main configuration structure for the PBS exporter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// PBS server configuration
    pub pbs: PbsConfig,

    /// Exporter server configuration
    pub exporter: ExporterConfig,
}

fn default_verify_tls() -> bool {
    true
}

fn default_timeout() -> u64 {
    5
}

fn default_snapshot_history_limit() -> usize {
    0 // 0 means all snapshots (full timeline)
}

fn default_listen_address() -> String {
    "0.0.0.0:9101".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Settings {
    /// Load configuration from a file and environment variables.
    ///
    /// Loads settings from a TOML configuration file (if provided) and merges
    /// with environment variables prefixed with `PBS_EXPORTER_`. Environment
    /// variables override file settings and use double underscores (`__`) as
    /// separators for nested values (e.g., `PBS_EXPORTER__PBS__ENDPOINT`).
    ///
    /// # Arguments
    ///
    /// * `config_path` - Optional path to TOML configuration file. If the file
    ///   doesn't exist, only environment variables will be used.
    ///
    /// # Returns
    ///
    /// Returns a validated `Settings` struct containing PBS and exporter
    /// configuration, or an error if configuration is invalid or missing
    /// required values.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The configuration file exists but cannot be parsed
    /// - Environment variables cannot be parsed
    /// - Required fields (endpoint, token_id, token_secret) are missing or empty
    /// - The resulting configuration fails validation
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pbs_exporter::config::Settings;
    ///
    /// // Load from file
    /// let settings = Settings::load(Some("config/default.toml")).unwrap();
    /// println!("PBS endpoint: {}", settings.pbs.endpoint);
    ///
    /// // Load from environment variables only
    /// std::env::set_var("PBS_EXPORTER__PBS__ENDPOINT", "https://pbs.local:8007");
    /// std::env::set_var("PBS_EXPORTER__PBS__TOKEN_ID", "user@pam!token");
    /// std::env::set_var("PBS_EXPORTER__PBS__TOKEN_SECRET", "secret");
    /// let settings = Settings::load(None).unwrap();
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn load(config_path: Option<&str>) -> Result<Self> {
        let mut builder = config::Config::builder();

        // Add config file if provided
        if let Some(path) = config_path {
            if Path::new(path).exists() {
                builder = builder.add_source(config::File::with_name(path));
            }
        }

        // Add environment variables with PBS_EXPORTER_ prefix
        builder = builder.add_source(
            config::Environment::with_prefix("PBS_EXPORTER")
                .separator("__")
                .try_parsing(true),
        );

        let config = builder.build()?;
        let settings: Settings = config.try_deserialize()?;

        settings.validate()?;
        Ok(settings)
    }

    /// Validate configuration settings.
    fn validate(&self) -> Result<()> {
        if self.pbs.endpoint.is_empty() {
            return Err(PbsError::Config(config::ConfigError::Message(
                "PBS endpoint cannot be empty".to_string(),
            )));
        }

        if self.pbs.token_id.is_empty() || self.pbs.token_secret.is_empty() {
            return Err(PbsError::Config(config::ConfigError::Message(
                "PBS API token credentials are required".to_string(),
            )));
        }

        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pbs: PbsConfig {
                endpoint: "https://localhost:8007".to_string(),
                token_id: String::new(),
                token_secret: String::new(),
                verify_tls: default_verify_tls(),
                ca_cert_path: None,
                timeout_seconds: default_timeout(),
                snapshot_history_limit: default_snapshot_history_limit(),
            },
            exporter: ExporterConfig {
                listen_address: default_listen_address(),
                log_level: default_log_level(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.pbs.endpoint, "https://localhost:8007");
        assert_eq!(settings.exporter.listen_address, "0.0.0.0:9101");
        assert!(settings.pbs.verify_tls);
        assert_eq!(settings.pbs.snapshot_history_limit, 0);
    }

    #[test]
    fn omitted_tls_setting_enables_certificate_verification() {
        let config = config::Config::builder()
            .add_source(config::File::from_str(
                "endpoint = 'https://localhost:8007'\ntoken_id = 'test@pam!token'\ntoken_secret = 'dummy'",
                config::FileFormat::Toml,
            ))
            .build().unwrap();
        let pbs: PbsConfig = config.try_deserialize().unwrap();
        assert!(pbs.verify_tls);
    }

    #[test]
    fn test_validation_fails_without_credentials() {
        let settings = Settings::default();
        assert!(settings.validate().is_err());
    }
}
