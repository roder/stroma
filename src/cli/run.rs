use super::config::{
    default_config_path, default_passphrase_path, default_store_path, StromaConfig,
};
use super::passphrase::{read_passphrase, PassphraseSource};
use std::path::PathBuf;
use stroma::crypto::StromaKeyring;

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

    // TODO: Wire keyring into BotConfig
    // When StromaBot is instantiated, create BotConfig with:
    //   - identity_masking_key: *keyring.identity_masking_key()
    //   - voter_pepper: *keyring.voter_pepper()
    //
    // The bot will then pass these keys to:
    //   - MemberResolver (identity_masking_key)
    //   - BootstrapManager (identity_masking_key)
    //   - PollManager (voter_pepper)

    // Ensure keyring is not optimized away (will be used when bot is implemented)
    let _ = keyring.identity_masking_key();

    // TODO: Implement actual bot service
    // This will:
    // 1. Load configuration from config_path
    // 2. Initialize Signal connection with passphrase-encrypted store
    // 3. Start embedded Freenet kernel
    // 4. Create BotConfig with keyring-derived keys
    // 5. Enter await bootstrap or normal operation mode
    // 6. Process Signal messages and update Freenet state

    println!("❌ Bot service not yet implemented");
    println!("The bot would now:");
    println!("  ✅ Connect to Signal");
    println!("  ✅ Initialize embedded Freenet kernel");
    println!("  ⏳ Await member-initiated bootstrap...");

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
            None,
            None,
            None,
        )
        .await;
        std::env::remove_var("STROMA_DB_PASSPHRASE");

        assert!(result.is_ok());
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
            None,
            Some("@alice".to_string()),
            None,
        )
        .await;
        std::env::remove_var("STROMA_DB_PASSPHRASE");

        assert!(result.is_ok());
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

        assert!(result.is_ok());
        // Config should now exist
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
            None,
            None,
            Some(passphrase_path.to_string_lossy().to_string()),
        )
        .await;

        assert!(result.is_ok());
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

        assert!(result.is_ok());
    }
}
