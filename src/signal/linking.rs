//! Signal Device Linking
//!
//! Implements secondary device linking via QR code.
//! Bot links to existing Signal account (operator's responsibility).
//!
//! See: .beads/signal-integration.bead § Signal Account & Device Linking

use super::traits::*;
use std::path::Path;

/// Device linking configuration
pub struct LinkingConfig {
    /// Device name shown in Signal app
    pub device_name: String,

    /// Path to protocol store
    pub store_path: std::path::PathBuf,

    /// Operator-provided passphrase
    pub passphrase: String,
}

impl LinkingConfig {
    pub fn new(
        device_name: impl Into<String>,
        store_path: impl AsRef<Path>,
        passphrase: impl Into<String>,
    ) -> Self {
        Self {
            device_name: device_name.into(),
            store_path: store_path.as_ref().to_path_buf(),
            passphrase: passphrase.into(),
        }
    }
}

/// Link as secondary device
///
/// Displays QR code for operator to scan with Signal app.
/// Returns when linking is complete and bot receives ACI/PNI identity.
pub async fn link_secondary_device(_config: LinkingConfig) -> SignalResult<()> {
    // TODO: Implement Presage device linking
    // 1. Create StromaStore
    // 2. Call Manager::link_secondary_device()
    // 3. Display QR code via qr2term
    // 4. Wait for linking confirmation
    // 5. Save identity to store

    Err(SignalError::NotImplemented(
        "link_secondary_device".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linking_config() {
        let config = LinkingConfig::new("Stroma Bot", "/tmp/store.bin", "secure-passphrase");

        assert_eq!(config.device_name, "Stroma Bot");
        assert_eq!(
            config.store_path,
            std::path::PathBuf::from("/tmp/store.bin")
        );
        assert_eq!(config.passphrase, "secure-passphrase");
    }

    #[tokio::test]
    async fn test_link_not_implemented() {
        let config = LinkingConfig::new("Test Bot", "/tmp/test.store", "pass");
        let result = link_secondary_device(config).await;
        assert!(result.is_err());
    }
}
