//! Configuration and error types for drift-auth.
//!
//! - AuthConfig: Configuration for multi-signature authentication
//! - Error types: ConfigError, AuthError, SignatureError, TimeoutError
//! - Metrics: Tracking for auth operations

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub use thiserror::Error;

/// === Item 43: AuthConfig struct ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable multi-signature authentication
    pub enabled: bool,

    /// Threshold for signature aggregation (m-of-n)
    pub threshold: usize,

       /// Maximum age for auth messages (seconds)
    #[serde(default)]
    pub max_message_age_secs: u64,

    /// Path to store node identity
    #[serde(default)]
    pub identity_path: Option<PathBuf>,

    /// Key rotation interval (seconds, 0 = disabled)
    #[serde(default)]
    pub key_rotation_interval_secs: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 3,
            max_message_age_secs: 300,
            identity_path: None,
            key_rotation_interval_secs: 0,
        }
    }
}

/// === Item 44: Load auth config from TOML/JSON ===
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("configuration file error: {0}")]
    Io(String),

    #[error("configuration parse error: {0}")]
    Parse(String),

    #[error("configuration validation error: {0}")]
    Validation(String),
}

impl AuthConfig {
    pub fn from_toml(path: &std::path::Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io(e.to_string()))?;
        toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    pub fn from_json(path: &std::path::Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io(e.to_string()))?;
        serde_json::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// === Item 45: Validate configuration ===
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.threshold == 0 {
            return Err(ConfigError::Validation("threshold must be >= 1".to_string()));
        }
        Ok(())
    }
}

/// === Item 45: AuthError, SignatureError, TimeoutError ===
#[derive(Error, Debug, Clone)]
pub enum AuthError {
    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("signature error: {0}")]
    Signature(#[from] SignatureError),

    #[error("timeout: {0}")]
    Timeout(#[from] TimeoutError),
}

#[derive(Error, Debug, Clone)]
pub enum SignatureError {
    #[error("signature verification failed")]
    VerificationFailed,

    #[error("invalid signature format")]
    InvalidFormat,

    #[error("signature aggregation failed: {0}")]
    AggregationFailed(String),
}

#[derive(Error, Debug, Clone)]
pub enum TimeoutError {
    #[error("operation timed out after {0} seconds")]
    TimedOut(u64),

    #[error("coordinator timeout: missing {0} signatures")]
    MissingSignatures(usize),
}

/// === Item 46: User-friendly error messages (via thiserror Display) ===

/// === Item 47: Metrics for tracking auth operations ===
pub struct AuthMetrics {
    pub nodes_authenticated: AtomicUsize,
    pub consensus_time_ms: AtomicU64,
}

impl AuthMetrics {
    pub fn new() -> Self {
        Self {
            nodes_authenticated: AtomicUsize::new(0),
            consensus_time_ms: AtomicU64::new(0),
        }
    }

    pub fn increment_authenticated(&self) {
        self.nodes_authenticated.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_consensus_time(&self, duration: Duration) {
        self.consensus_time_ms.store(duration.as_millis() as u64, Ordering::Relaxed);
    }

    pub fn nodes_authenticated(&self) -> usize {
        self.nodes_authenticated.load(Ordering::Relaxed)
    }

    pub fn consensus_time_ms(&self) -> u64 {
        self.consensus_time_ms.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.threshold, 3);
        assert_eq!(config.max_message_age_secs, 300);
        assert!(config.identity_path.is_none());
        assert_eq!(config.key_rotation_interval_secs, 0);
    }

    #[test]
    fn test_auth_config_from_json_valid() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("auth_config_test.json");

        let json = r#"{"enabled":true,"threshold":5,"max_message_age_secs":600}"#;
        std::fs::write(&config_path, json).unwrap();

        let config = AuthConfig::from_json(&config_path).unwrap();
        assert!(config.enabled);
        assert_eq!(config.threshold, 5);
        assert_eq!(config.max_message_age_secs, 600);

        std::fs::remove_file(&config_path).unwrap();
    }

    #[test]
    fn test_auth_config_from_json_invalid() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("auth_config_invalid.json");

        let json = "not valid json {";
        std::fs::write(&config_path, json).unwrap();

        let result = AuthConfig::from_json(&config_path);
        assert!(result.is_err());

        std::fs::remove_file(&config_path).unwrap();
    }

    #[test]
    fn test_auth_config_from_toml_valid() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("auth_config_test.toml");

        let toml = r#"
enabled = true
threshold = 4
max_message_age_secs = 450
"#;
        std::fs::write(&config_path, toml).unwrap();

        let config = AuthConfig::from_toml(&config_path).unwrap();
        assert!(config.enabled);
        assert_eq!(config.threshold, 4);
        assert_eq!(config.max_message_age_secs, 450);

        std::fs::remove_file(&config_path).unwrap();
    }

    #[test]
    fn test_auth_config_from_toml_invalid() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("auth_config_invalid.toml");

        let toml = "not valid toml {{{{";
        std::fs::write(&config_path, toml).unwrap();

        let result = AuthConfig::from_toml(&config_path);
        assert!(result.is_err());

        std::fs::remove_file(&config_path).unwrap();
    }

    #[test]
    fn test_auth_config_validate_threshold_zero() {
        let config = AuthConfig { enabled: true, threshold: 0, max_message_age_secs: 300, identity_path: None, key_rotation_interval_secs: 0 };
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_config_validate_threshold_positive() {
        let config = AuthConfig { enabled: true, threshold: 3, max_message_age_secs: 300, identity_path: None, key_rotation_interval_secs: 0 };
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::Io("file not found: some path".to_string());
        let display = err.to_string();
        assert!(display.contains("configuration file error"));
        assert!(display.contains("file not found"));
    }

    #[test]
    fn test_auth_error_display() {
        let err = AuthError::AuthFailed("invalid signature".to_string());
        let display = err.to_string();
        assert!(display.contains("authentication failed"));
        assert!(display.contains("invalid signature"));
    }

    #[test]
    fn test_signature_error_display() {
        let err = SignatureError::VerificationFailed;
        let display = err.to_string();
        assert!(display.contains("signature verification failed"));
    }

    #[test]
    fn test_timeout_error_display() {
        let err = TimeoutError::TimedOut(30);
        let display = err.to_string();
        assert!(display.contains("timed out"));
        assert!(display.contains("30 seconds"));
    }

    #[test]
    fn test_timeout_error_missing_signatures() {
        let err = TimeoutError::MissingSignatures(2);
        let display = err.to_string();
        assert!(display.contains("missing 2 signatures"));
    }

    #[test]
    fn test_auth_metrics_new() {
        let metrics = AuthMetrics::new();
        assert_eq!(metrics.nodes_authenticated(), 0);
        assert_eq!(metrics.consensus_time_ms(), 0);
    }

    #[test]
    fn test_auth_metrics_increment() {
        let metrics = AuthMetrics::new();
        metrics.increment_authenticated();
        metrics.increment_authenticated();
        assert_eq!(metrics.nodes_authenticated(), 2);
    }

    #[test]
    fn test_auth_metrics_record_time() {
        let metrics = AuthMetrics::new();
        metrics.record_consensus_time(Duration::from_millis(150));
        assert_eq!(metrics.consensus_time_ms(), 150);
    }

    #[test]
    fn test_auth_config_serialization_roundtrip() {
        let config1 = AuthConfig {
            enabled: true,
            threshold: 5,
            max_message_age_secs: 600,
            identity_path: Some(PathBuf::from("/path/to/identity")),
            key_rotation_interval_secs: 86400,
        };

        let json = serde_json::to_string(&config1).unwrap();
        let config2: AuthConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config1.enabled, config2.enabled);
        assert_eq!(config1.threshold, config2.threshold);
        assert_eq!(config1.max_message_age_secs, config2.max_message_age_secs);
        assert_eq!(config1.key_rotation_interval_secs, config2.key_rotation_interval_secs);
    }
}