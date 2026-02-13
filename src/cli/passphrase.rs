use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// Modes for passphrase delivery, checked in order
#[derive(Debug)]
pub enum PassphraseSource {
    /// From --passphrase-file /path/to/key (container-native)
    File(String),
    /// From stdin prompt (interactive, masked input)
    Stdin,
    /// From STROMA_DB_PASSPHRASE env var (fallback, warned as insecure)
    EnvVar,
}

/// Generate a BIP-39 passphrase (24-word recovery phrase, 256 bits entropy)
///
/// This should be displayed once on stderr at link/register time.
pub fn generate_passphrase() -> String {
    use bip39::{Language, Mnemonic};
    use rand::RngCore;

    // Generate 32 bytes (256 bits) of entropy for 24-word mnemonic
    let mut entropy = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut entropy);

    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .expect("Failed to generate BIP-39 mnemonic");

    mnemonic.to_string()
}

/// Read existing passphrase from file, env, or stdin (paste from vault)
///
/// Used by commands that need to access an existing encrypted store.
///
/// # Arguments
/// * `source` - The passphrase source to use
/// * `prompt` - Optional prompt message for stdin mode
pub fn read_passphrase(
    source: PassphraseSource,
    prompt: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    match source {
        PassphraseSource::File(path) => {
            if !Path::new(&path).exists() {
                return Err(format!("Passphrase file not found: {}", path).into());
            }

            let passphrase = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read passphrase file: {}", e))?
                .trim()
                .to_string();

            if passphrase.is_empty() {
                return Err("Passphrase file is empty".into());
            }

            Ok(passphrase)
        }
        PassphraseSource::Stdin => {
            let prompt_msg = prompt.unwrap_or("Enter passphrase: ");
            let passphrase = rpassword::prompt_password(prompt_msg)
                .map_err(|e| format!("Failed to read passphrase from stdin: {}", e))?;

            if passphrase.is_empty() {
                return Err("Passphrase cannot be empty".into());
            }

            Ok(passphrase)
        }
        PassphraseSource::EnvVar => {
            eprintln!("⚠️  WARNING: Using STROMA_DB_PASSPHRASE env var is insecure");
            eprintln!("   This mode should only be used for testing or in secure environments");
            eprintln!("   Consider using --passphrase-file instead");
            eprintln!();

            std::env::var("STROMA_DB_PASSPHRASE")
                .map_err(|_| "STROMA_DB_PASSPHRASE env var not set".into())
        }
    }
}

/// Determine passphrase source from CLI arguments
///
/// Returns the appropriate PassphraseSource based on:
/// 1. If passphrase_file is Some, use File
/// 2. If STROMA_DB_PASSPHRASE is set, use EnvVar
/// 3. Otherwise, use Stdin
pub fn determine_passphrase_source(passphrase_file: Option<String>) -> PassphraseSource {
    if let Some(file) = passphrase_file {
        PassphraseSource::File(file)
    } else if std::env::var("STROMA_DB_PASSPHRASE").is_ok() {
        PassphraseSource::EnvVar
    } else {
        PassphraseSource::Stdin
    }
}

/// Obtain passphrase for new store creation (register/link-device)
///
/// If passphrase is provided via file/env, use it (restore from backup).
/// Otherwise, generate a new BIP-39 passphrase.
///
/// # Arguments
/// * `source` - The passphrase source to use
/// * `save_to_path` - If provided and source is Stdin, save the generated passphrase here
///
/// # Returns
/// * Passphrase to use for encrypting the new store
pub fn obtain_passphrase_for_new_store(
    source: PassphraseSource,
    save_to_path: Option<&Path>,
) -> Result<String, Box<dyn std::error::Error>> {
    match source {
        PassphraseSource::File(_) | PassphraseSource::EnvVar => {
            // Operator provided existing passphrase (e.g., from backup)
            read_passphrase(source, None)
        }
        PassphraseSource::Stdin => {
            // Generate new passphrase
            let passphrase = generate_passphrase();

            // Save to file if path provided
            if let Some(path) = save_to_path {
                save_passphrase_to_file(&passphrase, path)?;
                display_generated_passphrase(&passphrase, Some(path));
            } else {
                display_generated_passphrase(&passphrase, None);
            }

            // Prompt user to confirm they've saved it
            println!("Please confirm you have saved the passphrase securely.");
            println!("Press Enter to continue or Ctrl+C to abort...");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            Ok(passphrase)
        }
    }
}

/// Display generated passphrase on stderr with clear warnings
///
/// This is called once at register/link time to show the operator their
/// BIP-39 recovery phrase. The passphrase is critical for accessing the
/// encrypted Signal protocol store.
///
/// If `saved_path` is provided, it indicates the passphrase was saved to a file
/// and the warning will include the file location.
pub fn display_generated_passphrase(passphrase: &str, saved_path: Option<&Path>) {
    // ANSI escape codes for terminal formatting
    const BOLD: &str = "\x1b[1m";
    const RESET: &str = "\x1b[0m";
    const DIM: &str = "\x1b[2m";

    // Box width (79 chars fits standard 80-col terminal)
    const BOX_WIDTH: usize = 79;

    eprintln!();
    eprintln!("{}", "═".repeat(BOX_WIDTH));
    eprintln!("🔑 {BOLD}CRITICAL: Database Passphrase (SAVE THIS SECURELY){RESET}");
    eprintln!("{}", "═".repeat(BOX_WIDTH));
    eprintln!();

    eprintln!();
    eprintln!("{}", "─".repeat(BOX_WIDTH));
    eprintln!("  {DIM}Copy-paste version (single line):{RESET}");
    eprintln!();
    eprintln!("  {BOLD}{passphrase}{RESET}");
    eprintln!();
    eprintln!("{}", "═".repeat(BOX_WIDTH));
    eprintln!();
    eprintln!("This is a BIP-39 24-word recovery phrase for your encrypted");
    eprintln!("Signal protocol store. You will need it to:");
    eprintln!("  • Restore the bot after server failure");
    eprintln!("  • Access backups");
    eprintln!("  • Run the bot service");
    eprintln!();

    if let Some(path) = saved_path {
        eprintln!("📁 SAVED TO: {BOLD}{}{RESET}", path.display());
        eprintln!();
        eprintln!("⚠️  SECURITY WARNING:");
        eprintln!("  • The passphrase file is readable only by you (mode 0600)");
        eprintln!("  • BACK UP this file to a secure location NOW");
        eprintln!("  • If you lose this file and the passphrase, your Signal");
        eprintln!("    identity is PERMANENTLY LOST");
        eprintln!("  • Consider copying to a password manager or secure backup");
        eprintln!("  • Never commit this file to version control");
        eprintln!("  • For production: use --passphrase-file with a secrets manager");
    } else {
        eprintln!("⚠️  SECURITY CRITICAL:");
        eprintln!("  • This passphrase is shown ONCE - write it down NOW");
        eprintln!("  • Store in a password manager or secure location");
        eprintln!("  • WITHOUT it, your Signal identity is PERMANENTLY LOST");
        eprintln!("  • Never commit this to version control or share it");
    }
    eprintln!();
    eprintln!("═══════════════════════════════════════════════════════════════════════════════");
    eprintln!();

    // Force flush to ensure message is displayed
    let _ = io::stderr().flush();
}

/// Save passphrase to a file with restrictive permissions (0600)
///
/// Creates the file with owner-only read/write permissions to protect
/// the sensitive passphrase from other users on the system.
///
/// # Arguments
/// * `passphrase` - The passphrase to save
/// * `path` - The file path to save to
///
/// # Returns
/// * `Ok(())` if successful
/// * `Err` if file creation or write fails
pub fn save_passphrase_to_file(
    passphrase: &str,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create passphrase directory: {}", e))?;
    }

    // Create file with restrictive permissions (0600 = owner read/write only)
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| {
            format!(
                "Failed to create passphrase file '{}': {}",
                path.display(),
                e
            )
        })?;

    writeln!(file, "{}", passphrase)
        .map_err(|e| format!("Failed to write passphrase to '{}': {}", path.display(), e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn test_generate_passphrase() {
        let passphrase = generate_passphrase();

        // BIP-39 24-word mnemonic should have 23 spaces (24 words)
        assert_eq!(passphrase.split_whitespace().count(), 24);

        // Generate another to ensure randomness
        let passphrase2 = generate_passphrase();
        assert_ne!(passphrase, passphrase2);
    }

    #[test]
    fn test_read_passphrase_from_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "test passphrase").unwrap();

        let source = PassphraseSource::File(temp_file.path().to_string_lossy().to_string());
        let result = read_passphrase(source, None);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test passphrase");
    }

    #[test]
    fn test_read_passphrase_file_not_found() {
        let source = PassphraseSource::File("/nonexistent/file".to_string());
        let result = read_passphrase(source, None);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_read_passphrase_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        // Don't write anything - file is empty

        let source = PassphraseSource::File(temp_file.path().to_string_lossy().to_string());
        let result = read_passphrase(source, None);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_determine_passphrase_source_file() {
        let source = determine_passphrase_source(Some("/path/to/key".to_string()));
        assert!(matches!(source, PassphraseSource::File(_)));
    }

    #[test]
    fn test_determine_passphrase_source_env_var() {
        std::env::set_var("STROMA_DB_PASSPHRASE", "test");
        let source = determine_passphrase_source(None);
        assert!(matches!(source, PassphraseSource::EnvVar));
        std::env::remove_var("STROMA_DB_PASSPHRASE");
    }

    #[test]
    fn test_determine_passphrase_source_stdin() {
        std::env::remove_var("STROMA_DB_PASSPHRASE");
        let source = determine_passphrase_source(None);
        assert!(matches!(source, PassphraseSource::Stdin));
    }

    #[test]
    fn test_save_passphrase_to_file() {
        let temp_dir = TempDir::new().unwrap();
        let passphrase_path = temp_dir.path().join("passphrase.txt");
        let test_passphrase = "test mnemonic words here";

        save_passphrase_to_file(test_passphrase, &passphrase_path).unwrap();

        // Verify file exists and contains passphrase
        assert!(passphrase_path.exists());
        let contents = fs::read_to_string(&passphrase_path).unwrap();
        assert_eq!(contents.trim(), test_passphrase);

        // Verify permissions are 0600 (owner read/write only)
        let metadata = fs::metadata(&passphrase_path).unwrap();
        let permissions = metadata.permissions();
        assert_eq!(permissions.mode() & 0o777, 0o600);
    }

    #[test]
    fn test_save_passphrase_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let passphrase_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("passphrase.txt");
        let test_passphrase = "test mnemonic";

        save_passphrase_to_file(test_passphrase, &passphrase_path).unwrap();

        assert!(passphrase_path.exists());
    }

    #[test]
    fn test_save_and_read_passphrase_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let passphrase_path = temp_dir.path().join("passphrase.txt");
        let original = generate_passphrase();

        // Save passphrase
        save_passphrase_to_file(&original, &passphrase_path).unwrap();

        // Read it back using read_passphrase
        let source = PassphraseSource::File(passphrase_path.to_string_lossy().to_string());
        let loaded = read_passphrase(source, None).unwrap();

        assert_eq!(original, loaded);
    }

    #[test]
    fn test_obtain_passphrase_from_existing_file() {
        // Test backup restore scenario: user provides existing passphrase via file
        let temp_dir = TempDir::new().unwrap();
        let passphrase_file = temp_dir.path().join("existing-passphrase.txt");
        let existing_passphrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        // Create a file with an existing passphrase (simulates backup restore)
        std::fs::write(&passphrase_file, existing_passphrase).unwrap();

        let source = PassphraseSource::File(passphrase_file.to_string_lossy().to_string());
        let result = obtain_passphrase_for_new_store(source, None);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), existing_passphrase);
    }

    #[test]
    fn test_obtain_passphrase_from_env_var() {
        // Test env var scenario for new store creation
        std::env::set_var("STROMA_DB_PASSPHRASE", "env-var-passphrase-test");

        let source = PassphraseSource::EnvVar;
        let result = obtain_passphrase_for_new_store(source, None);

        std::env::remove_var("STROMA_DB_PASSPHRASE");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "env-var-passphrase-test");
    }

    #[test]
    fn test_read_passphrase_env_var() {
        std::env::set_var("STROMA_DB_PASSPHRASE", "test-env-passphrase");

        let source = PassphraseSource::EnvVar;
        let result = read_passphrase(source, None);

        std::env::remove_var("STROMA_DB_PASSPHRASE");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-env-passphrase");
    }

    #[test]
    fn test_read_passphrase_env_var_not_set() {
        std::env::remove_var("STROMA_DB_PASSPHRASE");

        let source = PassphraseSource::EnvVar;
        let result = read_passphrase(source, None);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not set"));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;
    use proptest::test_runner::{Config as ProptestConfig, RngAlgorithm, TestRng, TestRunner};

    const PROPTEST_SEED: &[u8; 32] = b"stroma-passphrase-proptest-32byt";

    /// Generate BIP-39 passphrase with deterministic seed (test-only)
    fn generate_passphrase_with_seed(seed: u64) -> String {
        use bip39::{Language, Mnemonic};
        use rand::SeedableRng;

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut entropy = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rng, &mut entropy);

        let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
            .expect("Failed to generate BIP-39 mnemonic");

        mnemonic.to_string()
    }

    /// Property: All generated passphrases are valid BIP-39
    #[test]
    fn prop_passphrase_generation_always_valid_bip39() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = 0u64..10000u64;

        runner
            .run(&strategy, |seed| {
                let passphrase = generate_passphrase_with_seed(seed);

                // Verify it's valid BIP-39
                let result = bip39::Mnemonic::parse(&passphrase);
                prop_assert!(
                    result.is_ok(),
                    "Generated passphrase must be valid BIP-39: {}",
                    passphrase
                );

                // Verify word count (24 words for 256-bit entropy)
                let word_count = passphrase.split_whitespace().count();
                prop_assert_eq!(word_count, 24, "Must have exactly 24 words");

                Ok(())
            })
            .unwrap();
    }

    /// Property: Different seeds produce different passphrases
    #[test]
    fn prop_passphrase_generation_unique() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy =
            (0u64..1000u64, 0u64..1000u64).prop_filter("Seeds must differ", |(a, b)| a != b);

        runner
            .run(&strategy, |(seed1, seed2)| {
                let pass1 = generate_passphrase_with_seed(seed1);
                let pass2 = generate_passphrase_with_seed(seed2);

                prop_assert_ne!(
                    pass1,
                    pass2,
                    "Different seeds must produce different passphrases"
                );

                Ok(())
            })
            .unwrap();
    }

    /// Property: Passphrase determinism (same seed = same passphrase)
    #[test]
    fn prop_passphrase_determinism() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = 0u64..1000u64;

        runner
            .run(&strategy, |seed| {
                let pass1 = generate_passphrase_with_seed(seed);
                let pass2 = generate_passphrase_with_seed(seed);

                prop_assert_eq!(pass1, pass2, "Same seed must produce same passphrase");

                Ok(())
            })
            .unwrap();
    }
}
