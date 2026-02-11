use super::passphrase::{
    determine_passphrase_source, display_generated_passphrase, generate_passphrase,
    read_passphrase,
};
use stroma::signal::stroma_store::StromaStore;
use std::path::PathBuf;

/// Register a new Stroma bot
///
/// This command:
/// 1. Generates a BIP-39 24-word passphrase (displayed once on stderr)
/// 2. Creates an encrypted Signal protocol store
/// 3. Prepares the bot for device linking
pub async fn execute(
    store_path: Option<String>,
    passphrase_file: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Registering new Stroma bot...");
    println!();

    // Determine store path
    let store_path = store_path.unwrap_or_else(|| {
        let default_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("stroma")
            .join("signal-store");
        default_path.to_string_lossy().to_string()
    });

    println!("Store Path: {}", store_path);
    println!();

    // Check if store already exists
    if std::path::Path::new(&store_path).exists() {
        return Err(format!(
            "Store already exists at: {}\nUse 'stroma link-device' to link an existing store",
            store_path
        )
        .into());
    }

    // Determine passphrase source
    let source = determine_passphrase_source(passphrase_file);

    // Handle passphrase based on source
    let passphrase = match source {
        super::passphrase::PassphraseSource::File(_) | super::passphrase::PassphraseSource::EnvVar => {
            // Read existing passphrase from file or env var
            read_passphrase(source, None)?
        }
        super::passphrase::PassphraseSource::Stdin => {
            // Generate new passphrase and display it
            let new_passphrase = generate_passphrase();
            display_generated_passphrase(&new_passphrase);

            // Prompt user to confirm they've saved it
            println!("Please confirm you have saved the passphrase securely.");
            println!("Press Enter to continue or Ctrl+C to abort...");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            new_passphrase
        }
    };

    // Create parent directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(&store_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create store directory: {}", e))?;
    }

    // Initialize encrypted store
    println!("📦 Creating encrypted Signal protocol store...");
    let _store = StromaStore::open(&store_path, passphrase).await?;
    println!("✅ Store created successfully");
    println!();

    println!("✨ Registration complete!");
    println!();
    println!("Next steps:");
    println!("  1. Run 'stroma link-device' to link bot to Signal account");
    println!("  2. Use the same passphrase to access the store");
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_register_creates_store() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test-store");

        // Set env var to avoid interactive prompt
        std::env::set_var("STROMA_DB_PASSPHRASE", "test_passphrase");

        let result = execute(Some(store_path.to_string_lossy().to_string()), None).await;

        std::env::remove_var("STROMA_DB_PASSPHRASE");

        assert!(result.is_ok());
        assert!(store_path.exists());
    }

    #[tokio::test]
    async fn test_register_fails_if_store_exists() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test-store");

        // Create the store first
        std::fs::create_dir_all(&store_path).unwrap();

        std::env::set_var("STROMA_DB_PASSPHRASE", "test_passphrase");
        let result = execute(Some(store_path.to_string_lossy().to_string()), None).await;
        std::env::remove_var("STROMA_DB_PASSPHRASE");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_register_with_passphrase_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test-store");

        let mut passphrase_file = NamedTempFile::new().unwrap();
        writeln!(passphrase_file, "test passphrase from file").unwrap();

        let result = execute(
            Some(store_path.to_string_lossy().to_string()),
            Some(passphrase_file.path().to_string_lossy().to_string()),
        )
        .await;

        assert!(result.is_ok());
        assert!(store_path.exists());
    }
}
