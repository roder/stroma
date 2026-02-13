use super::config::{
    default_config_path, default_passphrase_path, default_store_path, StromaConfig,
};
use super::passphrase::{read_passphrase, PassphraseSource};
use presage::Manager;
use std::path::PathBuf;
use stroma::crypto::StromaKeyring;
use stroma::freenet::MockFreenetClient;
use stroma::signal::traits::{GroupId, ServiceId};
use stroma::signal::{BotConfig, LibsignalClient, StromaBot, StromaStore};

/// Run the bot service
///
/// This command starts the Stroma bot service with the specified configuration.
/// The bot will connect to Signal, initialize the embedded Freenet kernel,
/// and await member-initiated bootstrap if this is a new group.
///
/// ## Configuration Loading
///
/// Configuration is loaded from one of these sources (in order of precedence):
/// 1. `--config` flag if provided
/// 2. Config file adjacent to `--store-path` if provided
/// 3. Default config at `~/.local/share/stroma/config.toml`
///
/// If the config file doesn't exist, a default one is generated.
///
/// ## Passphrase Loading
///
/// Passphrase is loaded from one of these sources (in order of precedence):
/// 1. `--passphrase-file` flag if provided
/// 2. `STROMA_DB_PASSPHRASE` environment variable
/// 3. Passphrase file adjacent to config (`passphrase.txt`)
/// 4. Interactive prompt (stdin)
///
/// ## Key Derivation
///
/// All cryptographic keys are derived from the 24-word BIP-39 mnemonic passphrase
/// using HKDF-SHA256 with domain separation. See `crypto::keyring::StromaKeyring`
/// for the key hierarchy.
pub async fn execute(
    config_path: Option<String>,
    store_path: Option<String>,
    bootstrap_contact: Option<String>,
    passphrase_file: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with security-hardened log filtering.
    //
    // SECURITY: External crates (presage, libsignal-service, sqlx) log cleartext
    // Signal UUIDs at DEBUG/INFO level. Per security-constraints.bead §1:
    //   "NEVER log Signal IDs to disk, console, or any output"
    //
    // In RELEASE builds: We hardcode a WARN cap on external crates that
    // RUST_LOG cannot override. RUST_LOG only controls stroma's own log level.
    //
    // In DEBUG builds: RUST_LOG controls all crates (no security cap) to enable
    // development troubleshooting with full presage/libsignal traces.
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    #[cfg(not(debug_assertions))]
    {
        // RELEASE BUILD: Security cap enforced - external crates limited to WARN
        use tracing_subscriber::filter::Targets;

        let security_cap = Targets::new()
            .with_target("presage", tracing::Level::WARN)
            .with_target("libsignal_service", tracing::Level::WARN)
            .with_target("libsignal_protocol", tracing::Level::WARN)
            .with_target("sqlx", tracing::Level::WARN)
            .with_target("websocket", tracing::Level::WARN)
            .with_default(tracing::Level::TRACE);

        // Use try_init to avoid panic when called multiple times (e.g., in tests)
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(env_filter)
            .with(security_cap)
            .try_init();
    }

    #[cfg(debug_assertions)]
    {
        // DEBUG BUILD: No security cap - RUST_LOG controls everything
        eprintln!("⚠️  DEBUG BUILD: Log security caps disabled.");
        eprintln!("   External crates may log cleartext Signal UUIDs.");
        eprintln!("   Use release builds for production.\n");

        // Use try_init to avoid panic when called multiple times (e.g., in tests)
        let _ = tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(env_filter)
            .try_init();
    }

    println!("🚀 Starting Stroma bot service...");
    println!();

    // Determine store path (use provided or default)
    let store_path = store_path
        .map(PathBuf::from)
        .unwrap_or_else(default_store_path);

    // Determine config path (use provided, or derive from store path)
    let config_path = config_path
        .map(PathBuf::from)
        .unwrap_or_else(|| default_config_path(&store_path));

    println!("Config: {}", config_path.display());
    println!("Store: {}", store_path.display());

    // Load or create configuration
    let _config = if config_path.exists() {
        StromaConfig::load(&config_path)?
    } else {
        // Generate default config
        println!();
        println!("📝 No config file found. Creating default configuration...");
        StromaConfig::create_default(&config_path, &store_path)?;
        println!("   Created: {}", config_path.display());
        StromaConfig::load(&config_path)?
    };
    // TODO: Use config when bot service is fully implemented

    if let Some(contact) = &bootstrap_contact {
        println!("Bootstrap Contact: {}", contact);
        println!("Will prompt {} to initiate bootstrap", contact);
    } else {
        println!("No bootstrap contact specified");
        println!("Any member can initiate bootstrap with /create-group");
    }

    println!();

    // Determine passphrase source
    // Priority: --passphrase-file > env var > default passphrase file > stdin
    let source = if let Some(file) = passphrase_file {
        PassphraseSource::File(file)
    } else if std::env::var("STROMA_DB_PASSPHRASE").is_ok() {
        PassphraseSource::EnvVar
    } else {
        // Check for default passphrase file adjacent to config
        let default_passphrase = default_passphrase_path(&store_path);
        if default_passphrase.exists() {
            println!("📁 Using passphrase from: {}", default_passphrase.display());
            PassphraseSource::File(default_passphrase.to_string_lossy().to_string())
        } else {
            PassphraseSource::Stdin
        }
    };

    let passphrase = read_passphrase(
        source,
        Some("Enter 24-word mnemonic passphrase (or paste from password vault): "),
    )?;

    // Derive all cryptographic keys from the mnemonic
    // This provides the root of trust for:
    // - Identity masking (HMAC key)
    // - Voter deduplication (HMAC pepper)
    // - Chunk encryption/signing
    // - State encryption/signing
    let keyring = StromaKeyring::from_mnemonic(&passphrase)?;

    println!();
    println!("🔓 Opening encrypted store...");

    // Open StromaStore (creates signal.db + stroma.db)
    let store = StromaStore::open(store_path.as_path(), passphrase.clone()).await?;

    println!("📱 Loading registration data...");

    // Load registered Manager
    let manager = Manager::load_registered(store.clone()).await.map_err(|e| {
        format!(
            "Not registered. Run 'stroma register' or 'stroma link-device' first.\nError: {:?}",
            e
        )
    })?;

    // Extract service_id from registration data
    let reg_data = manager.registration_data();
    let service_id = ServiceId(reg_data.service_ids.aci.to_string());

    println!("✅ Registered as: {}", service_id.0);
    println!();

    // Create LibsignalClient with Manager (clones Manager for receive loop)
    let mut client = LibsignalClient::with_manager(service_id, store, manager);

    // Build BotConfig with keyring-derived keys
    // group_id will be set during /create-group bootstrap
    let config = BotConfig {
        group_id: GroupId(vec![]), // Set during bootstrap
        min_vouch_threshold: 2,
        identity_masking_key: *keyring.identity_masking_key(),
        voter_pepper: *keyring.voter_pepper(),
        contract_hash: None, // Freenet not yet available
    };

    // Create MockFreenetClient (real Freenet not yet integrated)
    let freenet = MockFreenetClient::new();

    println!("✅ Bot is running and connected to Signal");
    println!();
    println!("📬 Awaiting messages...");
    println!();
    println!("To initiate bootstrap:");
    println!("  1. Send a PM to this bot from your Signal account");
    println!("  2. Send: /create-group \"Your Group Name\"");
    println!("  3. Follow the bot's instructions to add seed members");
    println!();
    println!("Press Ctrl+C to stop.");
    println!();

    // Run bot with graceful shutdown on Ctrl+C
    // Use LocalSet because presage Manager is !Send
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            // Start the persistent receive_messages stream (must be inside LocalSet
            // because it uses spawn_local). This creates a single websocket for
            // receiving that stays open, avoiding 4409 "Connected elsewhere" errors.
            client
                .start_receive_loop()
                .await
                .map_err(|e| format!("Failed to start receive loop: {}", e))?;

            // Create bot AFTER receive loop starts (client is moved into bot)
            let mut bot = StromaBot::new(client, freenet, config)?;

            tokio::select! {
                result = bot.run() => {
                    result?;
                }
                _ = tokio::signal::ctrl_c() => {
                    println!();
                    println!("🛑 Shutting down gracefully...");
                }
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Standard BIP-39 test mnemonic (24 words)
    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";

    #[tokio::test]
    async fn test_run_with_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("signal-store");
        let config_path = temp_dir.path().join("config.toml");

        // Create config file
        let config_content = format!("[signal]\nstore_path = \"{}\"", store_path.display());
        std::fs::write(&config_path, config_content).unwrap();

        std::env::set_var("STROMA_DB_PASSPHRASE", TEST_MNEMONIC);
        let result = execute(
            Some(config_path.to_string_lossy().to_string()),
            Some(store_path.to_string_lossy().to_string()),
            None,
            None,
        )
        .await;
        std::env::remove_var("STROMA_DB_PASSPHRASE");

        // Should fail because no registration data exists
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Not registered") || err_msg.contains("register"));
    }

    #[tokio::test]
    async fn test_run_with_bootstrap_contact() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("signal-store");
        let config_path = temp_dir.path().join("config.toml");

        let config_content = format!("[signal]\nstore_path = \"{}\"", store_path.display());
        std::fs::write(&config_path, config_content).unwrap();

        std::env::set_var("STROMA_DB_PASSPHRASE", TEST_MNEMONIC);
        let result = execute(
            Some(config_path.to_string_lossy().to_string()),
            Some(store_path.to_string_lossy().to_string()),
            Some("@alice".to_string()),
            None,
        )
        .await;
        std::env::remove_var("STROMA_DB_PASSPHRASE");

        // Should fail because no registration data exists
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Not registered") || err_msg.contains("register"));
    }

    #[tokio::test]
    async fn test_run_creates_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("signal-store");
        let config_path = temp_dir.path().join("config.toml");

        // Config doesn't exist yet
        assert!(!config_path.exists());

        std::env::set_var("STROMA_DB_PASSPHRASE", TEST_MNEMONIC);
        let result = execute(
            None,
            Some(store_path.to_string_lossy().to_string()),
            None,
            None,
        )
        .await;
        std::env::remove_var("STROMA_DB_PASSPHRASE");

        // Should fail because no registration data exists
        assert!(result.is_err());
        // Config should have been created before the error
        assert!(config_path.exists());
    }

    #[tokio::test]
    async fn test_run_with_passphrase_file() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("signal-store");
        let config_path = temp_dir.path().join("config.toml");
        let passphrase_path = temp_dir.path().join("passphrase.txt");

        // Create config
        let config_content = format!("[signal]\nstore_path = \"{}\"", store_path.display());
        std::fs::write(&config_path, config_content).unwrap();

        // Create passphrase file
        std::fs::write(&passphrase_path, TEST_MNEMONIC).unwrap();

        let result = execute(
            Some(config_path.to_string_lossy().to_string()),
            Some(store_path.to_string_lossy().to_string()),
            None,
            Some(passphrase_path.to_string_lossy().to_string()),
        )
        .await;

        // Should fail because no registration data exists
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Not registered") || err_msg.contains("register"));
    }

    #[tokio::test]
    async fn test_run_uses_default_passphrase_file() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("signal-store");
        let config_path = temp_dir.path().join("config.toml");
        let passphrase_path = temp_dir.path().join("passphrase.txt");

        // Create config
        let config_content = format!("[signal]\nstore_path = \"{}\"", store_path.display());
        std::fs::write(&config_path, config_content).unwrap();

        // Create passphrase file at default location
        std::fs::write(&passphrase_path, TEST_MNEMONIC).unwrap();

        // Run without specifying passphrase file - should find it automatically
        let result = execute(
            Some(config_path.to_string_lossy().to_string()),
            Some(store_path.to_string_lossy().to_string()),
            None,
            None,
        )
        .await;

        // Should fail because no registration data exists
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Not registered") || err_msg.contains("register"));
    }
}
