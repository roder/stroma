use super::passphrase::{determine_passphrase_source, read_passphrase};
use std::path::PathBuf;

/// Link bot as secondary device to Signal account
///
/// This command displays a QR code that the operator scans with their Signal app
/// to link the bot as a secondary device.
pub async fn execute(
    device_name: String,
    store_path: Option<String>,
    servers: String,
    passphrase_file: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 Linking Stroma bot as secondary device...");
    println!();
    println!("Device Name: {}", device_name);

    let store_path = store_path.unwrap_or_else(|| {
        let default_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("stroma")
            .join("signal-store");
        default_path.to_string_lossy().to_string()
    });

    println!("Store Path: {}", store_path);
    println!("Servers: {}", servers);
    println!();

    // Read passphrase for opening encrypted store
    let source = determine_passphrase_source(passphrase_file);
    let _passphrase = read_passphrase(source, Some("Enter database passphrase: "))?;

    // TODO: Implement actual Signal device linking
    // This will use presage to:
    // 1. Generate provisioning URL
    // 2. Display QR code in terminal
    // 3. Wait for primary device to scan and approve
    // 4. Save Signal protocol store to store_path

    println!("❌ Signal integration not yet implemented");
    println!("This command will display a QR code for Signal device linking.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_link_device_with_custom_path() {
        // Set env var to avoid interactive prompt
        std::env::set_var("STROMA_DB_PASSPHRASE", "test_passphrase");

        let result = execute(
            "Test Bot".to_string(),
            Some("/tmp/test-store".to_string()),
            "production".to_string(),
            None,
        )
        .await;

        std::env::remove_var("STROMA_DB_PASSPHRASE");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_link_device_with_default_path() {
        std::env::set_var("STROMA_DB_PASSPHRASE", "test_passphrase");

        let result = execute(
            "Test Bot".to_string(),
            None,
            "production".to_string(),
            None,
        )
        .await;

        std::env::remove_var("STROMA_DB_PASSPHRASE");
        assert!(result.is_ok());
    }
}
