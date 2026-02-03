use std::path::PathBuf;

/// Link bot as secondary device to Signal account
///
/// This command displays a QR code that the operator scans with their Signal app
/// to link the bot as a secondary device.
pub async fn execute(
    device_name: String,
    store_path: Option<String>,
    servers: String,
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
        let result = execute(
            "Test Bot".to_string(),
            Some("/tmp/test-store".to_string()),
            "production".to_string(),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_link_device_with_default_path() {
        let result = execute("Test Bot".to_string(), None, "production".to_string()).await;

        assert!(result.is_ok());
    }
}
