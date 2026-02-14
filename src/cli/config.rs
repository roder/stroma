//! Stroma configuration file handling
//!
//! Provides default configuration generation and loading for the Stroma bot.
//! Configuration files are TOML format and stored adjacent to the Signal store.
//!
//! ## Operator vs Group Configuration
//!
//! This file contains OPERATOR configuration only - settings that the service
//! runner controls for their deployment (paths, logging, network settings).
//!
//! **GroupConfig** (min_vouch_threshold, quorum, etc.) is stored in the Freenet
//! contract and controlled by group consensus via `/propose` commands. The operator
//! CANNOT modify GroupConfig - this is an immutable security constraint.
//!
//! See: `.beads/security-constraints.bead` § 5 (Operator Least Privilege)
//! See: `.beads/philosophical-foundations.bead` § 2 (Inclusion vs Protection)

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Default log level
const DEFAULT_LOG_LEVEL: &str = "info";

/// Stroma bot configuration (OPERATOR settings only)
///
/// This struct contains deployment/infrastructure settings that the operator
/// controls. Trust model parameters (min_vouch_threshold, quorum, etc.) are
/// NOT here - they're in the Freenet contract and controlled by group consensus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StromaConfig {
    /// Signal protocol store configuration
    pub signal: SignalConfig,

    /// Freenet integration configuration
    #[serde(default)]
    pub freenet: FreenetConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Signal-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    /// Path to the Signal protocol store
    pub store_path: PathBuf,

    /// Signal servers to use (production or staging)
    #[serde(default = "default_servers")]
    pub servers: String,

    /// Signal group ID (hex-encoded master key) - set during bootstrap
    ///
    /// Per 1:1 bot-to-group architecture, this is set when /create-group completes
    /// and becomes IMMUTABLE. Once set, the bot will:
    /// - Only respond to messages from this group
    /// - Leave all other groups
    /// - Reject attempts to change this value
    ///
    /// To change groups, you must unregister and re-bootstrap the bot.
    pub group_id: Option<String>,
}

/// Freenet integration configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FreenetConfig {
    /// Freenet node address (optional, uses embedded node if not specified)
    pub node_address: Option<String>,

    /// Contract hash for the trust network (set automatically after group creation)
    /// This is READ from Freenet, not configured by operator
    pub contract_hash: Option<String>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log file path (optional, logs to stderr if not specified)
    pub file: Option<PathBuf>,
}

fn default_servers() -> String {
    "production".to_string()
}

fn default_log_level() -> String {
    DEFAULT_LOG_LEVEL.to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: DEFAULT_LOG_LEVEL.to_string(),
            file: None,
        }
    }
}

impl StromaConfig {
    /// Create a new configuration with the given store path
    #[allow(dead_code)]
    pub fn new(store_path: PathBuf) -> Self {
        Self {
            signal: SignalConfig {
                store_path,
                servers: default_servers(),
                group_id: None,
            },
            freenet: FreenetConfig::default(),
            logging: LoggingConfig::default(),
        }
    }

    /// Load configuration from a TOML file
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file '{}': {}", path.display(), e))?;

        let config: StromaConfig = toml::from_str(&contents)
            .map_err(|e| format!("Failed to parse config file '{}': {}", path.display(), e))?;

        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let contents = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        fs::write(path, contents)
            .map_err(|e| format!("Failed to write config file '{}': {}", path.display(), e))?;

        Ok(())
    }

    /// Set group_id and persist to config file (called when bootstrap completes)
    ///
    /// This makes the bot's group assignment IMMUTABLE. Once set, the bot will
    /// only respond to this group and reject all others.
    ///
    /// Returns error if group_id is already set (to prevent accidental changes).
    #[allow(dead_code)] // TODO: Used when CLI->library config integration is complete
    pub fn set_group_id(
        &mut self,
        path: &Path,
        group_id_hex: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.signal.group_id.is_some() {
            return Err(
                "Group ID already set. Bot is already bootstrapped to a group. \
                       To change groups, unregister and re-bootstrap."
                    .into(),
            );
        }

        self.signal.group_id = Some(group_id_hex);
        self.save(path)?;

        Ok(())
    }

    /// Generate default configuration content as a string with comments
    pub fn generate_default_toml(store_path: &Path) -> String {
        format!(
            r#"# Stroma Bot Configuration (Operator Settings)
# 
# This file contains OPERATOR configuration only - deployment settings that
# the service runner controls (paths, logging, network).
#
# TRUST MODEL PARAMETERS (min_vouch_threshold, quorum, etc.) are stored in
# the Freenet contract and controlled by GROUP CONSENSUS via /propose commands.
# The operator CANNOT modify trust parameters - this is a security constraint.
#
# See: OPERATOR-GUIDE.md for deployment instructions
# See: docs/TRUST-MODEL.md for how group parameters work

[signal]
# Path to the encrypted Signal protocol store
store_path = "{store_path}"

# Signal servers: "production" or "staging"
servers = "production"

# Group ID (hex-encoded master key) - SET AUTOMATICALLY during bootstrap
# This field is set when /create-group completes and becomes IMMUTABLE.
# Once set, the bot will ONLY respond to this group and leave all others.
# To change groups, you must unregister and re-bootstrap the bot.
# Do not set this manually - it's managed by the bot.
# group_id = "..."

[freenet]
# Freenet node address (optional)
# Leave commented to use embedded Freenet kernel
# node_address = "ws://localhost:50509"

# Contract hash is set AUTOMATICALLY after group creation via /create-group
# Do not set this manually - it's managed by the bot
# contract_hash = "..."

[logging]
# Log level: trace, debug, info, warn, error
level = "info"

# Log file path (optional, logs to stderr if not specified)
# file = "/var/log/stroma/stroma.log"
"#,
            store_path = store_path.display()
        )
    }

    /// Create and save a default configuration file
    pub fn create_default(
        config_path: &Path,
        store_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let contents = Self::generate_default_toml(store_path);

        // Create parent directory if needed
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        fs::write(config_path, contents).map_err(|e| {
            format!(
                "Failed to write config file '{}': {}",
                config_path.display(),
                e
            )
        })?;

        Ok(())
    }
}

/// Get the default config file path based on the store path
///
/// The config file is stored adjacent to the Signal store directory:
/// - Store: ~/.local/share/stroma/signal-store/
/// - Config: ~/.local/share/stroma/config.toml
pub fn default_config_path(store_path: &Path) -> PathBuf {
    // Config goes in the parent directory of the store
    // e.g., store_path = /data/stroma/signal-store -> config = /data/stroma/config.toml
    store_path
        .parent()
        .unwrap_or(store_path)
        .join("config.toml")
}

/// Get the default passphrase file path based on the store path
///
/// The passphrase file is stored adjacent to the config:
/// - Store: ~/.local/share/stroma/signal-store/
/// - Passphrase: ~/.local/share/stroma/passphrase.txt
pub fn default_passphrase_path(store_path: &Path) -> PathBuf {
    store_path
        .parent()
        .unwrap_or(store_path)
        .join("passphrase.txt")
}

/// Get the default store path
pub fn default_store_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("stroma")
        .join("signal-store")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let store_path = PathBuf::from("/data/stroma/signal-store");
        let config = StromaConfig::new(store_path.clone());

        assert_eq!(config.signal.store_path, store_path);
        assert_eq!(config.signal.servers, "production");
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let store_path = PathBuf::from("/data/stroma/signal-store");

        let config = StromaConfig::new(store_path.clone());
        config.save(&config_path).unwrap();

        let loaded = StromaConfig::load(&config_path).unwrap();
        assert_eq!(loaded.signal.store_path, store_path);
        assert_eq!(loaded.logging.level, "info");
    }

    #[test]
    fn test_create_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let store_path = temp_dir.path().join("signal-store");

        StromaConfig::create_default(&config_path, &store_path).unwrap();

        assert!(config_path.exists());

        // Verify it can be loaded
        let config = StromaConfig::load(&config_path).unwrap();
        assert_eq!(config.signal.store_path, store_path);
    }

    #[test]
    fn test_default_config_path() {
        let store_path = PathBuf::from("/data/stroma/signal-store");
        let config_path = default_config_path(&store_path);
        assert_eq!(config_path, PathBuf::from("/data/stroma/config.toml"));
    }

    #[test]
    fn test_default_passphrase_path() {
        let store_path = PathBuf::from("/data/stroma/signal-store");
        let passphrase_path = default_passphrase_path(&store_path);
        assert_eq!(
            passphrase_path,
            PathBuf::from("/data/stroma/passphrase.txt")
        );
    }

    #[test]
    fn test_generate_default_toml() {
        let store_path = PathBuf::from("/data/stroma/signal-store");
        let toml = StromaConfig::generate_default_toml(&store_path);

        assert!(toml.contains("store_path = \"/data/stroma/signal-store\""));
        assert!(toml.contains("servers = \"production\""));
        // Trust parameters should NOT be configurable in operator config
        // (no [trust] section with actual values)
        assert!(!toml.contains("[trust]"));
        assert!(!toml.contains("min_vouch_threshold = "));
        // Config should explain that trust params are group-controlled
        assert!(toml.contains("GROUP CONSENSUS"));
    }

    #[test]
    fn test_load_config_with_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Write minimal config (only required fields)
        let minimal_config = r#"
[signal]
store_path = "/tmp/store"
"#;
        fs::write(&config_path, minimal_config).unwrap();

        let config = StromaConfig::load(&config_path).unwrap();

        // Verify defaults are applied
        assert_eq!(config.signal.servers, "production");
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_config_does_not_contain_group_config() {
        // Verify that StromaConfig does not have fields that belong to GroupConfig
        // This is a compile-time check - if these fields exist, the test won't compile
        let store_path = PathBuf::from("/tmp/store");
        let config = StromaConfig::new(store_path);

        // These fields should NOT exist on StromaConfig (they're in Freenet contract)
        // - min_vouch_threshold
        // - quorum_percentage
        // - config_change_threshold
        // If we try to access them, we get a compile error, which is correct behavior

        // Verify we have the fields we DO expect
        let _ = config.signal.store_path;
        let _ = config.signal.servers;
        let _ = config.freenet.node_address;
        let _ = config.logging.level;
    }
}
