use super::passphrase::{determine_passphrase_source, read_passphrase};
use std::path::{Path, PathBuf};

/// Backup Signal protocol store
///
/// This command creates a secure backup of the Signal protocol store,
/// which contains the ACI identity keypair critical for recovery.
/// Losing this store means losing access to the trust network forever.
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

    // Read passphrase for opening encrypted store
    let source = determine_passphrase_source(passphrase_file);
    let _passphrase = read_passphrase(source, Some("Enter database passphrase: "))?;

    // TODO: Implement actual backup
    // This will:
    // 1. Create a tarball of the Signal protocol store
    // 2. Optionally encrypt the backup
    // 3. Save to output_path with timestamp
    // 4. Verify backup integrity

    println!("❌ Backup functionality not yet implemented");
    println!();
    println!("⚠️  CRITICAL: This store contains your ACI identity key");
    println!("   Without it, you CANNOT decrypt your persistence fragments");
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

        // Should fail because store doesn't exist
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Signal protocol store not found"));
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

        // Should fail since default store path won't exist
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Signal protocol store not found"));
    }
}
