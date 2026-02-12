use super::config::{default_config_path, default_passphrase_path, StromaConfig};
use super::passphrase::{
    determine_passphrase_source, obtain_passphrase_for_new_store, PassphraseSource,
};
use presage::libsignal_service::configuration::SignalServers;
use std::path::PathBuf;
use stroma::signal::linking::{link_secondary_device, LinkingConfig};

/// Determine whether to save the passphrase to a file
///
/// Passphrase is saved when it's being generated (stdin mode).
/// When passphrase is provided via file or env var, it's already persisted
/// somewhere, so we don't need to save it again.
///
/// # Arguments
/// * `source` - The passphrase source
///
/// # Returns
/// * `true` if passphrase should be saved (generated from stdin)
/// * `false` if passphrase was provided externally
pub fn should_save_passphrase(source: &PassphraseSource) -> bool {
    matches!(source, PassphraseSource::Stdin)
}

/// Parse server environment string to SignalServers enum
///
/// # Arguments
/// * `servers` - Server environment string (e.g., "production", "staging")
///
/// # Returns
/// * `Ok(SignalServers)` - Parsed server environment
/// * `Err(String)` - Error message for invalid input
pub fn parse_server_environment(servers: &str) -> Result<SignalServers, String> {
    match servers.to_lowercase().as_str() {
        "production" | "prod" => Ok(SignalServers::Production),
        "staging" => Ok(SignalServers::Staging),
        _ => Err(format!(
            "Invalid server environment: {}. Use 'production' or 'staging'",
            servers
        )),
    }
}

/// Link bot as secondary device to Signal account
///
/// This command displays a QR code that the operator scans with their Signal app
/// to link the bot as a secondary device. Creates a new encrypted store with a
/// BIP-39 passphrase (generated or provided).
pub async fn execute(
    device_name: String,
    store_path: Option<String>,
    servers: String,
    passphrase_file: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine store path
    let store_path = store_path.unwrap_or_else(|| {
        let default_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("stroma")
            .join("signal-store");
        default_path.to_string_lossy().to_string()
    });

    // Determine if we should save the passphrase to a file
    let source = determine_passphrase_source(passphrase_file);
    let save_passphrase = should_save_passphrase(&source);
    let passphrase_path = default_passphrase_path(&PathBuf::from(&store_path));

    // Obtain passphrase for new encrypted store
    // If generating a new passphrase, save it to the default location
    let passphrase = obtain_passphrase_for_new_store(
        source,
        if save_passphrase {
            Some(&passphrase_path)
        } else {
            None
        },
    )?;

    // Parse server environment
    let signal_servers = parse_server_environment(&servers)?;

    // Create linking config and link device
    let store_path_buf = PathBuf::from(&store_path);
    let config = LinkingConfig::new(
        device_name,
        store_path_buf.clone(),
        passphrase,
        signal_servers,
    );

    link_secondary_device(config).await?;

    // Create default config file if we generated a passphrase
    if save_passphrase {
        let config_path = default_config_path(&store_path_buf);
        StromaConfig::create_default(&config_path, &store_path_buf)?;
        println!("📝 Created config: {}", config_path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_save_passphrase_stdin() {
        let source = PassphraseSource::Stdin;
        assert!(should_save_passphrase(&source));
    }

    #[test]
    fn test_should_save_passphrase_file() {
        let source = PassphraseSource::File("/path/to/file".to_string());
        assert!(!should_save_passphrase(&source));
    }

    #[test]
    fn test_should_save_passphrase_env() {
        let source = PassphraseSource::EnvVar;
        assert!(!should_save_passphrase(&source));
    }

    #[test]
    fn test_parse_server_environment_production() {
        let prod_servers = vec!["production", "prod", "Production", "PROD"];
        for server in prod_servers {
            let result = parse_server_environment(server);
            assert!(result.is_ok(), "Failed to parse: {}", server);
            assert!(matches!(result.unwrap(), SignalServers::Production));
        }
    }

    #[test]
    fn test_parse_server_environment_staging() {
        let staging_servers = vec!["staging", "Staging", "STAGING"];
        for server in staging_servers {
            let result = parse_server_environment(server);
            assert!(result.is_ok(), "Failed to parse: {}", server);
            assert!(matches!(result.unwrap(), SignalServers::Staging));
        }
    }

    #[test]
    fn test_parse_server_environment_invalid() {
        let invalid_servers = vec!["invalid", "prod1", "stage", ""];
        for server in invalid_servers {
            let result = parse_server_environment(server);
            assert!(result.is_err(), "Should fail for: {}", server);
            assert!(result.unwrap_err().contains("Invalid server environment"));
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

    // Note: Full E2E linking tests require real Signal infrastructure
    // and are tested manually per testing-standards.bead § Manual E2E Testing
}
