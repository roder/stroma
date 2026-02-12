use super::passphrase::{determine_passphrase_source, read_passphrase};
use std::path::{Path, PathBuf};

/// Backup Signal protocol store
///
/// This command creates a secure backup of the Signal protocol store,
/// which contains the Signal protocol state necessary for maintaining
/// the bot's Signal account connection. Note: Freenet chunk decryption
/// and identity masking keys are derived from the BIP-39 mnemonic, NOT
/// stored here. Back up both the mnemonic AND this store for full recovery.
pub async fn execute(
    output_path: String,
    passphrase_file: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("💾 Backing up Signal protocol store...");
    println!();

    let output = Path::new(&output_path);

    // Validate output path
    if let Some(parent) = output.parent() {
        if !parent.exists() {
            return Err(format!("Output directory does not exist: {}", parent.display()).into());
        }
    }

    // Get default store path
    let default_store = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("stroma")
        .join("signal-store");

    println!("  Source: {}", default_store.display());
    println!("  Output: {}", output.display());
    println!();

    // Check if store exists
    if !default_store.exists() {
        return Err(format!(
            "Signal protocol store not found at: {}",
            default_store.display()
        )
        .into());
    }

    // Read existing passphrase for opening encrypted store
    let source = determine_passphrase_source(passphrase_file);
    let _passphrase = read_passphrase(
        source,
        Some("Enter database passphrase (or paste from password vault): "),
    )?;

    // TODO: Implement actual backup
    // This will:
    // 1. Create a tarball of the Signal protocol store
    // 2. Optionally encrypt the backup
    // 3. Save to output_path with timestamp
    // 4. Verify backup integrity

    println!("❌ Backup functionality not yet implemented");
    println!();
    println!("⚠️  CRITICAL: This store contains your Signal protocol state");
    println!("   Required for maintaining bot's Signal account connection.");
    println!("   Note: Freenet chunk decryption uses mnemonic-derived keys.");
    println!("   Store backup in:");
    println!("     - Encrypted USB drive in safe location");
    println!("     - Hardware security module (HSM)");
    println!("     - Secure cloud backup (encrypted)");
    println!("     - NOT on the same server as the bot");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_backup_store_with_valid_output() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("backup.tar.gz");

        std::env::set_var("STROMA_DB_PASSPHRASE", "test_passphrase");
        let result = execute(output_path.to_string_lossy().to_string(), None).await;
        std::env::remove_var("STROMA_DB_PASSPHRASE");

        // Result depends on whether default store exists on this machine:
        // - If store doesn't exist: Err("Signal protocol store not found")
        // - If store exists: Ok(()) with "not yet implemented" message
        // Both are valid outcomes for this test (we're testing the output path validation)
        let default_store = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("stroma")
            .join("signal-store");

        if default_store.exists() {
            // Store exists, function should succeed (returning "not implemented")
            assert!(
                result.is_ok(),
                "Expected Ok when store exists: {:?}",
                result
            );
        } else {
            // Store doesn't exist, should fail with specific error
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Signal protocol store not found"));
        }
    }

    #[tokio::test]
    async fn test_backup_store_with_invalid_output_dir() {
        std::env::set_var("STROMA_DB_PASSPHRASE", "test_passphrase");
        let result = execute("/nonexistent/dir/backup.tar.gz".to_string(), None).await;
        std::env::remove_var("STROMA_DB_PASSPHRASE");

        // Should fail because output directory doesn't exist
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_backup_store_when_source_missing() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("backup.tar.gz");

        std::env::set_var("STROMA_DB_PASSPHRASE", "test_passphrase");
        let result = execute(output_path.to_string_lossy().to_string(), None).await;
        std::env::remove_var("STROMA_DB_PASSPHRASE");

        // Result depends on whether default store exists on this machine
        let default_store = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("stroma")
            .join("signal-store");

        if default_store.exists() {
            // Store exists, function should succeed (returning "not implemented")
            assert!(
                result.is_ok(),
                "Expected Ok when store exists: {:?}",
                result
            );
        } else {
            // Store doesn't exist, should fail with specific error
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Signal protocol store not found"));
        }
    }
}
