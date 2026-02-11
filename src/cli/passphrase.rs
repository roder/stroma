use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

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

/// Read passphrase based on delivery mode priority
///
/// Checks in order:
/// 1. --passphrase-file /path/to/key (container-native)
/// 2. Stdin prompt (interactive, masked input)
/// 3. STROMA_DB_PASSPHRASE env var (fallback, warned as insecure)
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

/// Resolve store path with default fallback
///
/// Returns the provided store path or the default Signal protocol store location.
pub fn resolve_store_path(store_path: Option<String>) -> String {
    store_path.unwrap_or_else(|| {
        let default_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("stroma")
            .join("signal-store");
        default_path.to_string_lossy().to_string()
    })
}

/// Display generated passphrase on stderr with clear warnings
///
/// This is called once at register/link time to show the operator their
/// BIP-39 recovery phrase. The passphrase is critical for accessing the
/// encrypted Signal protocol store.
pub fn display_generated_passphrase(passphrase: &str) {
    eprintln!();
    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!("🔑 CRITICAL: Database Passphrase (SAVE THIS SECURELY)");
    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!();
    eprintln!("{}", passphrase);
    eprintln!();
    eprintln!("This is a BIP-39 24-word recovery phrase for your encrypted");
    eprintln!("Signal protocol store. You will need it to:");
    eprintln!("  • Restore the bot after server failure");
    eprintln!("  • Access backups");
    eprintln!("  • Run the bot service");
    eprintln!();
    eprintln!("⚠️  SECURITY CRITICAL:");
    eprintln!("  • This passphrase is shown ONCE - write it down NOW");
    eprintln!("  • Store in a password manager or secure location");
    eprintln!("  • WITHOUT it, your Signal identity is PERMANENTLY LOST");
    eprintln!("  • Never commit this to version control or share it");
    eprintln!();
    eprintln!("═══════════════════════════════════════════════════════════════");
    eprintln!();

    // Force flush to ensure message is displayed
    let _ = io::stderr().flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

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
}
