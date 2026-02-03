use std::path::Path;

/// Run the bot service
///
/// This command starts the Stroma bot service with the specified configuration.
/// The bot will connect to Signal, initialize the embedded Freenet kernel,
/// and await member-initiated bootstrap if this is a new group.
pub async fn execute(
    config_path: String,
    bootstrap_contact: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Stroma bot service...");
    println!();
    println!("Config: {}", config_path);

    // Validate config file exists
    if !Path::new(&config_path).exists() {
        return Err(format!("Config file not found: {}", config_path).into());
    }

    if let Some(contact) = &bootstrap_contact {
        println!("Bootstrap Contact: {}", contact);
        println!("Will prompt {} to initiate bootstrap", contact);
    } else {
        println!("No bootstrap contact specified");
        println!("Any member can initiate bootstrap with /create-group");
    }

    println!();

    // TODO: Implement actual bot service
    // This will:
    // 1. Load configuration from config_path
    // 2. Initialize Signal connection
    // 3. Start embedded Freenet kernel
    // 4. Enter await bootstrap or normal operation mode
    // 5. Process Signal messages and update Freenet state

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
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_run_with_valid_config() {
        // Create a temporary config file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[signal]").unwrap();
        writeln!(temp_file, "store_path = \"/tmp/store\"").unwrap();

        let result = execute(
            temp_file.path().to_string_lossy().to_string(),
            None,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_with_bootstrap_contact() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[signal]").unwrap();

        let result = execute(
            temp_file.path().to_string_lossy().to_string(),
            Some("@alice".to_string()),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_with_missing_config() {
        let result = execute(
            "/nonexistent/config.toml".to_string(),
            None,
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
