## 10. Configuration (Items 43-47)

**Create drift-auth/src/config.rs:**

```rust
// drift-auth/src/config.rs
// Checklist items: 43, 44, 45, 46, 47

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use thiserror::Error;

/// === Item 43: AuthConfig struct ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable multi-signature authentication
    pub enabled: bool,

    /// Threshold for signature aggregation (m-of-n)
    pub threshold: usize,

    /// Maximum age for auth messages (seconds)
    #[serde(default = "300")]
    pub max_message_age_secs: u64,

    /// Path to store node identity
    #[serde(default)]
    pub identity_path: Option<PathBuf>,

    /// Key rotation interval (seconds, 0 = disabled)
    #[serde(default = "0")]
    pub key_rotation_interval_secs: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,  // Opt-in
            threshold: 3,
            max_message_age_secs: 300,
            identity_path: None,
            key_rotation_interval_secs: 0,
        }
    }
}

/// Load auth config from TOML/JSON
impl AuthConfig {
    pub fn from_toml(path: &std::path::Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::Io(e))?;
        toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(e.to_string()))
    }

    pub fn from_json(path: &std::path::Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::Io(e))?;
        serde_json::from_str(&content)
            .map_err(|e| ConfigError::Parse(e.to_string()))
    }
}

/// === Item 45: Error types ===
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(std::io::Error),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("invalid config: {0}")]
    Validation(String),
}

/// === Item 46: User-friendly error messages ===
impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "configuration file error: {}", e),
            Self::Parse(e) => write!(f, "configuration parse error: {}", e),
            Self::Validation(e) => write!(f, "configuration validation error: {}", e),
        }
    }
}

/// Validate configuration
impl AuthConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.threshold == 0 {
            return Err(ConfigError::Validation(
                "threshold must be >= 1".to_string()
            ));
        }
        Ok(())
    }
}
```
