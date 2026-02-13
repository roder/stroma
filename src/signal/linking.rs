//! Signal Device Linking
//!
//! Implements secondary device linking via QR code.
//! Bot links to existing Signal account (operator's responsibility).
//!
//! See: .beads/signal-integration.bead § Signal Account & Device Linking

use super::stroma_store::StromaStore;
use super::traits::*;
use futures::channel::oneshot;
use presage::libsignal_service::configuration::SignalServers;
use presage::Manager;
use std::path::Path;

/// Device linking configuration
pub struct LinkingConfig {
    /// Device name shown in Signal app
    pub device_name: String,

    /// Path to protocol store (directory containing signal.db and stroma.db)
    pub store_path: std::path::PathBuf,

    /// Operator-provided passphrase (24-word BIP-39 mnemonic)
    pub passphrase: String,

    /// Signal server environment
    pub signal_servers: SignalServers,
}

impl LinkingConfig {
    pub fn new(
        device_name: impl Into<String>,
        store_path: impl AsRef<Path>,
        passphrase: impl Into<String>,
        signal_servers: SignalServers,
    ) -> Self {
        Self {
            device_name: device_name.into(),
            store_path: store_path.as_ref().to_path_buf(),
            passphrase: passphrase.into(),
            signal_servers,
        }
    }
}

/// Link as secondary device
///
/// Displays QR code for operator to scan with Signal app.
/// Returns when linking is complete and bot receives ACI/PNI identity.
///
/// # Flow
/// 1. Create encrypted StromaStore (signal.db + stroma.db)
/// 2. Call Manager::link_secondary_device() with oneshot channel
/// 3. Display QR code when provisioning URL is received
/// 4. Wait for linking to complete
/// 5. Return registered manager (identity saved to store)
///
/// # Errors
/// Returns error if:
/// - Store creation fails
/// - Linking is cancelled or times out
/// - Network errors during linking
pub async fn link_secondary_device(config: LinkingConfig) -> SignalResult<()> {
    println!("📱 Preparing to link Stroma as secondary device...");
    println!();

    // Create encrypted StromaStore (will create both signal.db and stroma.db)
    let store = StromaStore::open(&config.store_path, config.passphrase)
        .await
        .map_err(|e| SignalError::Store(format!("Failed to create store: {}", e)))?;

    println!(
        "✅ Encrypted store created at: {}",
        config.store_path.display()
    );
    println!();

    // Create oneshot channel for provisioning URL
    // Type annotation for clarity: the channel carries the provisioning URL
    let (provisioning_link_tx, provisioning_link_rx) = oneshot::channel();

    // Clone device_name for later display
    let device_name_display = config.device_name.clone();

    // Run linking and QR display concurrently
    let linking_result = tokio::join!(
        // Linking task: calls presage Manager to link device
        async {
            Manager::link_secondary_device(
                store,
                config.signal_servers,
                config.device_name,
                provisioning_link_tx,
            )
            .await
        },
        // QR display task: waits for URL and displays QR code
        async {
            match provisioning_link_rx.await {
                Ok(url) => {
                    println!("📱 Please scan this QR code with Signal on your phone:");
                    println!("   Signal → Settings → Linked Devices → Link New Device");
                    println!();

                    if let Err(e) = qr2term::print_qr(url.to_string()) {
                        eprintln!("⚠️  Failed to render QR code: {}", e);
                        println!("Please use the URL instead: {}", url);
                    }

                    println!();
                    println!("Or use this URL: {}", url);
                    println!();
                    println!("⏳ Waiting for you to scan the code...");
                }
                Err(_) => {
                    eprintln!("❌ Linking was cancelled");
                }
            }
        },
    );

    // Check linking result
    match linking_result.0 {
        Ok(manager) => {
            println!();
            println!("✅ Device linked successfully!");

            // Display account information
            let whoami = manager
                .whoami()
                .await
                .map_err(|e| SignalError::Protocol(format!("Failed to get account info: {}", e)))?;

            println!("   Device Name: {}", device_name_display);
            println!("   ACI: {}", whoami.aci);
            println!("   PNI: {}", whoami.pni);
            println!("   Phone: {}", whoami.number);
            println!();
            println!("🎉 You can now run 'stroma run' to start the bot!");

            Ok(())
        }
        Err(e) => Err(SignalError::Protocol(format!(
            "Device linking failed: {}",
            e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linking_config_production() {
        let config = LinkingConfig::new(
            "Stroma Bot",
            "/tmp/store.bin",
            "secure-passphrase",
            SignalServers::Production,
        );

        assert_eq!(config.device_name, "Stroma Bot");
        assert_eq!(
            config.store_path,
            std::path::PathBuf::from("/tmp/store.bin")
        );
        assert_eq!(config.passphrase, "secure-passphrase");
        assert!(matches!(config.signal_servers, SignalServers::Production));
    }

    #[test]
    fn test_linking_config_staging() {
        let config = LinkingConfig::new(
            "Test Bot",
            "/var/lib/stroma/store",
            "test-mnemonic-phrase",
            SignalServers::Staging,
        );

        assert_eq!(config.device_name, "Test Bot");
        assert_eq!(
            config.store_path,
            std::path::PathBuf::from("/var/lib/stroma/store")
        );
        assert_eq!(config.passphrase, "test-mnemonic-phrase");
        assert!(matches!(config.signal_servers, SignalServers::Staging));
    }

    #[test]
    fn test_linking_config_path_normalization() {
        // Test that paths are correctly converted to PathBuf
        let config = LinkingConfig::new(
            "Bot",
            "relative/path/to/store",
            "pass",
            SignalServers::Production,
        );

        assert_eq!(
            config.store_path,
            std::path::PathBuf::from("relative/path/to/store")
        );
    }

    #[test]
    fn test_linking_config_with_complex_device_name() {
        // Device names can have spaces and special characters
        let config = LinkingConfig::new(
            "My Stroma Bot (Server #2)",
            "/tmp/store",
            "pass",
            SignalServers::Production,
        );

        assert_eq!(config.device_name, "My Stroma Bot (Server #2)");
    }

    // Note: Actual linking tests require real Signal infrastructure
    // or complex mocking of presage Manager. Testing is done via
    // manual E2E validation per testing-standards.bead.
}
