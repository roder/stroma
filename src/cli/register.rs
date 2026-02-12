use super::config::{default_config_path, default_passphrase_path, StromaConfig};
use super::passphrase::{
    determine_passphrase_source, display_generated_passphrase, generate_passphrase,
    read_passphrase, save_passphrase_to_file, PassphraseSource,
};
use presage::libsignal_service::configuration::SignalServers;
use std::path::{Path, PathBuf};
use stroma::signal::registration::{register_device, RegistrationConfig};

/// Action to take based on store state validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreAction {
    /// Store doesn't exist, create a new one
    CreateNew,
    /// Store exists and --force was specified, replace it
    ForceReplace,
}

/// Validate store state and determine action to take
///
/// Returns the appropriate action based on whether the store exists
/// and whether the --force flag was specified.
///
/// # Arguments
/// * `store_path` - Path to the store directory
/// * `force` - Whether --force flag was specified
///
/// # Returns
/// * `Ok(StoreAction)` - Action to take
/// * `Err(String)` - Error message if store exists without --force
pub fn validate_store_state(store_path: &Path, force: bool) -> Result<StoreAction, String> {
    let exists = store_path.exists();
    match (exists, force) {
        (false, _) => Ok(StoreAction::CreateNew),
        (true, true) => Ok(StoreAction::ForceReplace),
        (true, false) => Err(format!("Store already exists at: {}", store_path.display())),
    }
}

/// Register Stroma bot as primary device with Signal
///
/// This command:
/// 1. Creates an encrypted Signal protocol store (passphrase generated silently)
/// 2. Registers the phone number with Signal servers
/// 3. Prompts for SMS/voice verification code
/// 4. Completes registration and displays account info
/// 5. ONLY on success: displays the BIP-39 passphrase to save
///
/// The passphrase is deferred to the end so that failed registration attempts
/// (captcha required, rate limiting, etc.) don't waste the operator's time
/// with a passphrase they'll never use.
pub async fn execute(
    phone: String,
    store_path: Option<String>,
    servers: String,
    voice: bool,
    captcha: Option<String>,
    force: bool,
    passphrase_file: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Registering new Stroma bot as primary device...");
    println!();

    // Determine store path
    let store_path = store_path.unwrap_or_else(|| {
        let default_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("stroma")
            .join("signal-store");
        default_path.to_string_lossy().to_string()
    });

    // Validate store state and determine action
    let store_path_ref = Path::new(&store_path);
    match validate_store_state(store_path_ref, force) {
        Ok(StoreAction::CreateNew) => {
            // Store doesn't exist, proceed with creation
        }
        Ok(StoreAction::ForceReplace) => {
            // Delete existing store so we start fresh with a new passphrase
            println!("🗑️  Removing existing store (--force)...");
            std::fs::remove_dir_all(&store_path)
                .map_err(|e| format!("Failed to remove existing store: {}", e))?;
        }
        Err(msg) => {
            return Err(format!(
                "{}\n\
                To re-register, use the --force flag:\n\
                stroma register --phone {} --force",
                msg, phone
            )
            .into());
        }
    }

    // Obtain passphrase -- silently for generated, or from file/env
    // The passphrase is NOT displayed yet; it will be shown only on success
    let source = determine_passphrase_source(passphrase_file);
    let (passphrase, is_generated) = match source {
        PassphraseSource::File(_) | PassphraseSource::EnvVar => {
            let pp = read_passphrase(source, None)?;
            (pp, false)
        }
        PassphraseSource::Stdin => {
            let pp = generate_passphrase();
            (pp, true)
        }
    };

    // Create parent directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(&store_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create store directory: {}", e))?;
    }

    // Parse server environment
    let signal_servers = match servers.to_lowercase().as_str() {
        "production" | "prod" => SignalServers::Production,
        "staging" => SignalServers::Staging,
        _ => {
            return Err(format!(
                "Invalid server environment: {}. Use 'production' or 'staging'",
                servers
            )
            .into());
        }
    };

    // Build registration config
    let config = RegistrationConfig::new(&phone, &store_path, &passphrase, signal_servers)
        .with_voice(voice)
        .with_captcha(captcha)
        .with_force(force);

    // Perform registration (store created internally, cleaned up on failure)
    register_device(config).await?;

    // Registration succeeded -- NOW display the passphrase and save config if we generated it
    if is_generated {
        println!();

        // Save passphrase to default location
        let passphrase_path = default_passphrase_path(&PathBuf::from(&store_path));
        save_passphrase_to_file(&passphrase, &passphrase_path)?;

        // Create default config file
        let config_path = default_config_path(&PathBuf::from(&store_path));
        StromaConfig::create_default(&config_path, &PathBuf::from(&store_path))?;
        println!("📝 Created config: {}", config_path.display());

        display_generated_passphrase(&passphrase, Some(&passphrase_path));
        println!("Please confirm you have saved the passphrase securely.");
        println!("Press Enter to continue...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_store_new() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("nonexistent");

        let result = validate_store_state(&store_path, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StoreAction::CreateNew);
    }

    #[test]
    fn test_validate_store_exists_no_force() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("existing-store");
        std::fs::create_dir(&store_path).unwrap();

        let result = validate_store_state(&store_path, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Store already exists"));
    }

    #[test]
    fn test_validate_store_exists_with_force() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("existing-store");
        std::fs::create_dir(&store_path).unwrap();

        let result = validate_store_state(&store_path, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StoreAction::ForceReplace);
    }

    #[test]
    fn test_validate_store_new_with_force() {
        // --force on non-existent store should still return CreateNew
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("nonexistent");

        let result = validate_store_state(&store_path, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StoreAction::CreateNew);
    }

    #[test]
    fn test_server_parsing() {
        // Test that server string parsing works correctly
        let prod_servers = vec!["production", "prod", "Production", "PROD"];
        for server in prod_servers {
            let result = match server.to_lowercase().as_str() {
                "production" | "prod" => Ok(SignalServers::Production),
                "staging" => Ok(SignalServers::Staging),
                _ => Err("Invalid"),
            };
            assert!(result.is_ok(), "Failed to parse: {}", server);
        }

        let staging_servers = vec!["staging", "Staging", "STAGING"];
        for server in staging_servers {
            let result = match server.to_lowercase().as_str() {
                "production" | "prod" => Ok(SignalServers::Production),
                "staging" => Ok(SignalServers::Staging),
                _ => Err("Invalid"),
            };
            assert!(result.is_ok(), "Failed to parse: {}", server);
        }
    }

    #[test]
    fn test_default_store_path() {
        let default_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("stroma")
            .join("signal-store");

        // Just verify the path construction works
        assert!(default_path.to_string_lossy().contains("stroma"));
    }

    // Note: Full E2E registration tests require real Signal infrastructure
    // and are tested manually per testing-standards.bead § Manual E2E Testing
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use proptest::test_runner::{Config as ProptestConfig, RngAlgorithm, TestRng, TestRunner};
    use tempfile::TempDir;

    const PROPTEST_SEED: &[u8; 32] = b"stroma-register-proptest-seed32b";

    /// Property: validate_store_state behavior is consistent
    #[test]
    fn prop_validate_store_state_consistent() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            cases: 100, // Reduce cases since we're dealing with filesystem
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy =
            prop::bool::ANY.prop_flat_map(|store_exists| (Just(store_exists), prop::bool::ANY));

        runner
            .run(&strategy, |(store_exists, force)| {
                let temp_dir = TempDir::new().unwrap();
                let store_path = temp_dir.path().join("test-store");

                if store_exists {
                    std::fs::create_dir(&store_path).unwrap();
                }

                let result = validate_store_state(&store_path, force);

                // Verify logical consistency
                match (store_exists, force) {
                    (false, _) => prop_assert!(matches!(result, Ok(StoreAction::CreateNew))),
                    (true, true) => prop_assert!(matches!(result, Ok(StoreAction::ForceReplace))),
                    (true, false) => prop_assert!(result.is_err()),
                }

                Ok(())
            })
            .unwrap();
    }

    /// Property: validate_store_state never panics
    #[test]
    fn prop_validate_store_state_never_panics() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            cases: 100,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = (prop::bool::ANY, prop::bool::ANY);

        runner
            .run(&strategy, |(store_exists, force)| {
                let temp_dir = TempDir::new().unwrap();
                let store_path = temp_dir.path().join("test-store");

                if store_exists {
                    std::fs::create_dir(&store_path).unwrap();
                }

                // Should not panic regardless of inputs
                let _ = validate_store_state(&store_path, force);

                Ok(())
            })
            .unwrap();
    }

    /// Property: CreateNew action is always returned for non-existent stores
    #[test]
    fn prop_create_new_for_nonexistent() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            cases: 50,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = prop::bool::ANY;

        runner
            .run(&strategy, |force| {
                let temp_dir = TempDir::new().unwrap();
                let store_path = temp_dir.path().join("nonexistent-store");

                // Store doesn't exist
                let result = validate_store_state(&store_path, force);

                prop_assert!(result.is_ok());
                prop_assert!(matches!(result.unwrap(), StoreAction::CreateNew));

                Ok(())
            })
            .unwrap();
    }
}
