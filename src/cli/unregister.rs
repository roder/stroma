use super::config::default_passphrase_path;
use super::passphrase::{read_passphrase, PassphraseSource};
use presage::manager::RegistrationType;
use presage::Manager;
use std::io::{self, Write};
use std::path::PathBuf;
use stroma::signal::stroma_store::StromaStore;

/// Unregister Stroma bot and clean up local data
///
/// This command handles two scenarios:
/// - **Primary device + --delete-account**: Permanently deletes the account from Signal servers
///   and removes all local data. The phone number can be re-registered afterwards.
/// - **Primary device (no flag)**: Clears local data only. The number is freed for re-registration
///   but server-side groups/contacts are preserved.
/// - **Secondary device**: Clears local data and provides instructions for unlinking from the
///   primary device (Signal protocol prevents secondary devices from self-unlinking).
pub async fn execute(
    store_path: Option<String>,
    passphrase_file: Option<String>,
    delete_account: bool,
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine store path
    let store_path = store_path.unwrap_or_else(|| {
        let default_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("stroma")
            .join("signal-store");
        default_path.to_string_lossy().to_string()
    });

    // Check if store exists
    let store_path_buf = PathBuf::from(&store_path);
    if !store_path_buf.exists() {
        return Err(format!("Store not found at: {}\nNothing to unregister.", store_path).into());
    }

    println!("🔓 Opening encrypted store...");

    // Determine passphrase source
    // Priority: --passphrase-file > env var > default passphrase file > stdin
    let source = if let Some(file) = passphrase_file {
        PassphraseSource::File(file)
    } else if std::env::var("STROMA_DB_PASSPHRASE").is_ok() {
        PassphraseSource::EnvVar
    } else {
        // Check for default passphrase file adjacent to store
        let default_passphrase = default_passphrase_path(&store_path_buf);
        if default_passphrase.exists() {
            println!("📁 Using passphrase from: {}", default_passphrase.display());
            PassphraseSource::File(default_passphrase.to_string_lossy().to_string())
        } else {
            PassphraseSource::Stdin
        }
    };

    let passphrase = read_passphrase(
        source,
        Some("Enter database passphrase (or paste from password vault): "),
    )?;

    // Open the store
    let store = StromaStore::open(&store_path, passphrase).await?;

    // Load registered manager to detect device type
    println!("📱 Detecting device type...");
    let manager = match Manager::load_registered(store.clone()).await {
        Ok(m) => m,
        Err(e) => {
            // If we can't load registration data, the store may not be fully registered
            return Err(format!(
                "Failed to load registration data: {:?}\n\
                The store may not be properly registered. You can manually delete the store directory:\n\
                  rm -rf {}",
                e, store_path
            )
            .into());
        }
    };

    let registration_type = manager.registration_type();

    match registration_type {
        RegistrationType::Primary => {
            handle_primary_device(manager, store, &store_path, delete_account, yes).await
        }
        RegistrationType::Secondary => handle_secondary_device(store, &store_path, yes).await,
    }
}

/// Handle unregistration for a primary device
async fn handle_primary_device(
    manager: Manager<StromaStore, presage::manager::Registered>,
    mut store: StromaStore,
    store_path: &str,
    delete_account: bool,
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if delete_account {
        // Full account deletion from Signal servers
        println!();
        println!("⚠️  WARNING: You are about to PERMANENTLY DELETE your Signal account!");
        println!();
        println!("This will:");
        println!("  • Delete your account from Signal servers");
        println!("  • Remove all groups, contacts, and messages (server-side)");
        println!("  • Allow the phone number to be re-registered with a fresh account");
        println!("  • Existing contacts will see 'safety number changed' on re-registration");
        println!();
        println!("This action is IRREVERSIBLE.");
        println!();

        if !yes && !confirm_action("Type 'DELETE' to confirm account deletion: ", "DELETE")? {
            println!("Aborted.");
            return Ok(());
        }

        println!();
        println!("🗑️  Deleting account from Signal servers...");

        // Delete account from Signal servers and clear local store
        manager.delete_account().await.map_err(|e| {
            format!(
                "Failed to delete account from Signal servers: {:?}\n\
                Your account may still exist on the server. Try again later.",
                e
            )
        })?;

        // Delete store directory
        delete_store_directory(store_path)?;

        println!("✅ Account deleted successfully!");
        println!();
        println!("Your Signal account has been permanently deleted.");
        println!(
            "You can re-register the phone number with 'stroma register' or 'stroma link-device'."
        );
    } else {
        // Local-only cleanup (preserves server-side data)
        println!();
        println!("⚠️  WARNING: You are about to remove all local Stroma data!");
        println!();
        println!("This will:");
        println!("  • Clear local Signal protocol store and Stroma data");
        println!("  • Remove all local encryption keys and session data");
        println!("  • Keep your server-side groups and contacts intact");
        println!();
        println!("Your Signal account will remain active on the server.");
        println!("Existing linked devices (if any) will continue to work.");
        println!();

        if !yes && !confirm_action("Type 'UNREGISTER' to confirm: ", "UNREGISTER")? {
            println!("Aborted.");
            return Ok(());
        }

        println!();
        println!("🧹 Clearing local data...");

        // Clear both databases
        store.clear_all().await?;

        // Delete store directory
        delete_store_directory(store_path)?;

        println!("✅ Local data removed successfully!");
        println!();
        println!("Your Signal account is still active on the server.");
        println!("You can re-link this device with 'stroma link-device'.");
    }

    Ok(())
}

/// Handle unregistration for a secondary device
async fn handle_secondary_device(
    mut store: StromaStore,
    store_path: &str,
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("📱 This is a SECONDARY device (linked from a primary Signal account).");
    println!();
    println!("Signal protocol does not allow secondary devices to unlink themselves.");
    println!("This command will remove local data only.");
    println!();
    println!("After removing local data, you MUST also unlink this device from your");
    println!("primary Signal app to complete the process:");
    println!();
    println!("  📱 On your phone: Signal → Settings → Linked Devices → Remove this device");
    println!();

    if !yes && !confirm_action("Type 'UNLINK' to confirm local data removal: ", "UNLINK")? {
        println!("Aborted.");
        return Ok(());
    }

    println!();
    println!("🧹 Clearing local data...");

    // Clear both databases
    store.clear_all().await?;

    // Delete store directory
    delete_store_directory(store_path)?;

    println!("✅ Local data removed successfully!");
    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("⚠️  IMPORTANT: Complete the unlinking process!");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("To fully unlink this device, open your primary Signal app:");
    println!();
    println!("  1. Open Signal on your phone");
    println!("  2. Go to Settings (⚙️)");
    println!("  3. Select 'Linked Devices'");
    println!("  4. Find and remove the Stroma device");
    println!();
    println!("Until you do this, the device will still appear in your linked devices list.");
    println!();

    Ok(())
}

/// Delete the store directory after clearing databases
fn delete_store_directory(store_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🗂️  Removing store directory...");

    std::fs::remove_dir_all(store_path).map_err(|e| {
        format!(
            "Failed to remove store directory: {}\n\
            You may need to manually delete: {}",
            e, store_path
        )
    })?;

    Ok(())
}

/// Parse user confirmation input against expected string
///
/// This is the pure logic portion of confirmation that can be unit tested.
/// Handles whitespace trimming and exact string matching.
///
/// # Arguments
/// * `input` - Raw user input (may include newlines/whitespace)
/// * `expected` - The exact string expected for confirmation
///
/// # Returns
/// * `true` if input matches expected after trimming
/// * `false` otherwise
pub fn parse_confirmation(input: &str, expected: &str) -> bool {
    input.trim() == expected
}

/// Prompt user for confirmation with a specific expected input
fn confirm_action(prompt: &str, expected: &str) -> Result<bool, Box<dyn std::error::Error>> {
    print!("{}", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(parse_confirmation(&input, expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_confirmation_exact_match() {
        assert!(parse_confirmation("DELETE", "DELETE"));
        assert!(parse_confirmation("UNREGISTER", "UNREGISTER"));
        assert!(parse_confirmation("UNLINK", "UNLINK"));
    }

    #[test]
    fn test_parse_confirmation_with_whitespace() {
        // Should handle trailing newline from stdin
        assert!(parse_confirmation("DELETE\n", "DELETE"));
        assert!(parse_confirmation("DELETE\r\n", "DELETE"));
        assert!(parse_confirmation("  DELETE  ", "DELETE"));
        assert!(parse_confirmation("\tDELETE\t", "DELETE"));
    }

    #[test]
    fn test_parse_confirmation_wrong_input() {
        assert!(!parse_confirmation("delete", "DELETE")); // case sensitive
        assert!(!parse_confirmation("DELET", "DELETE")); // partial
        assert!(!parse_confirmation("DELETE!", "DELETE")); // extra char
        assert!(!parse_confirmation("", "DELETE")); // empty
        assert!(!parse_confirmation("no", "DELETE")); // wrong word
    }

    #[test]
    fn test_parse_confirmation_empty_expected() {
        // Edge case: empty expected string
        assert!(parse_confirmation("", ""));
        assert!(parse_confirmation("  ", ""));
        assert!(!parse_confirmation("x", ""));
    }

    #[test]
    fn test_delete_store_directory_success() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("store-to-delete");
        std::fs::create_dir(&store_path).unwrap();

        // Create a file inside to verify recursive deletion
        std::fs::write(store_path.join("test.db"), b"test data").unwrap();

        let result = delete_store_directory(store_path.to_str().unwrap());
        assert!(result.is_ok());
        assert!(!store_path.exists());
    }

    #[test]
    fn test_delete_store_directory_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("nonexistent");

        let result = delete_store_directory(store_path.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to remove"));
    }

    #[test]
    fn test_store_path_default() {
        let default_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("stroma")
            .join("signal-store");

        // Just verify the path construction works
        assert!(default_path.to_string_lossy().contains("stroma"));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use proptest::test_runner::{Config as ProptestConfig, RngAlgorithm, TestRng, TestRunner};

    const PROPTEST_SEED: &[u8; 32] = b"stroma-unregister-proptest---32b";

    /// Property: parse_confirmation never panics
    #[test]
    fn prop_parse_confirmation_never_panics() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = (".*", ".*");

        runner
            .run(&strategy, |(input, expected)| {
                // Should handle any string inputs without panicking
                let _ = parse_confirmation(&input, &expected);
                Ok(())
            })
            .unwrap();
    }

    /// Property: Whitespace normalization is symmetric
    #[test]
    fn prop_whitespace_normalization() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = ("[A-Z]+", " *", " *");

        runner
            .run(&strategy, |(s, prefix_ws, suffix_ws)| {
                let input = format!("{}{}{}", prefix_ws, s, suffix_ws);
                let result = parse_confirmation(&input, &s);

                prop_assert!(result, "Whitespace should be trimmed: {:?}", input);

                Ok(())
            })
            .unwrap();
    }

    /// Property: Case sensitivity is preserved
    #[test]
    fn prop_case_sensitive() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = "[a-z]+";

        runner
            .run(&strategy, |s| {
                let uppercase = s.to_uppercase();
                let result = parse_confirmation(&s, &uppercase);

                prop_assert!(!result, "Confirmation must be case-sensitive");

                Ok(())
            })
            .unwrap();
    }

    /// Property: Exact match always succeeds (after trim)
    #[test]
    fn prop_exact_match_succeeds() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = "[A-Za-z0-9]+";

        runner
            .run(&strategy, |s| {
                let result = parse_confirmation(&s, &s);

                prop_assert!(result, "Exact match should always succeed");

                Ok(())
            })
            .unwrap();
    }

    /// Property: Empty strings match only empty expected
    #[test]
    fn prop_empty_string_behavior() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            cases: 50,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = " *";

        runner
            .run(&strategy, |whitespace| {
                // Empty input with empty expected should match
                let result_empty = parse_confirmation(&whitespace, "");
                prop_assert!(
                    result_empty,
                    "Whitespace-only input should match empty expected"
                );

                // Empty input with non-empty expected should not match
                let result_nonempty = parse_confirmation(&whitespace, "DELETE");
                prop_assert!(
                    !result_nonempty,
                    "Whitespace-only input should not match non-empty expected"
                );

                Ok(())
            })
            .unwrap();
    }

    /// Property: Substring does not match
    #[test]
    fn prop_substring_no_match() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            cases: 100,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = "[A-Z]{5,10}";

        runner
            .run(&strategy, |s| {
                if s.len() > 1 {
                    let substring = &s[..s.len() - 1];
                    let result = parse_confirmation(substring, &s);

                    prop_assert!(!result, "Substring should not match full string");
                }

                Ok(())
            })
            .unwrap();
    }
}
