use clap::{Parser, Subcommand};

pub mod backup_store;
pub mod link_device;
pub mod passphrase;
pub mod register;
pub mod run;
pub mod status;
pub mod verify;
pub mod version;

#[derive(Parser)]
#[command(name = "stroma")]
#[command(author = "Stroma Project")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Operator CLI for Stroma trust network bot", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Register a new Stroma bot with encrypted store
    Register {
        /// Path to Signal protocol store (optional, uses default if not specified)
        #[arg(long)]
        store_path: Option<String>,

        /// Path to file containing passphrase (container-native)
        #[arg(long)]
        passphrase_file: Option<String>,
    },

    /// Link bot as secondary device to Signal account
    LinkDevice {
        /// Device name shown in Signal's linked devices list
        #[arg(long)]
        device_name: String,

        /// Path to Signal protocol store (optional, uses default if not specified)
        #[arg(long)]
        store_path: Option<String>,

        /// Signal servers to use (production or staging)
        #[arg(long, default_value = "production")]
        servers: String,

        /// Path to file containing passphrase (container-native)
        #[arg(long)]
        passphrase_file: Option<String>,
    },

    /// Run the bot service
    Run {
        /// Path to config file
        #[arg(long)]
        config: String,

        /// Optional Signal contact to prompt for bootstrap initiation
        #[arg(long)]
        bootstrap_contact: Option<String>,

        /// Path to file containing passphrase (container-native)
        #[arg(long)]
        passphrase_file: Option<String>,
    },

    /// Check bot health and status
    Status,

    /// Verify installation integrity
    Verify,

    /// Backup Signal protocol store
    BackupStore {
        /// Output path for backup file
        #[arg(long)]
        output: String,

        /// Path to file containing passphrase (container-native)
        #[arg(long)]
        passphrase_file: Option<String>,
    },

    /// Display version information
    Version,
}

pub async fn execute(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Register {
            store_path,
            passphrase_file,
        } => register::execute(store_path, passphrase_file).await,
        Commands::LinkDevice {
            device_name,
            store_path,
            servers,
            passphrase_file,
        } => link_device::execute(device_name, store_path, servers, passphrase_file).await,
        Commands::Run {
            config,
            bootstrap_contact,
            passphrase_file,
        } => run::execute(config, bootstrap_contact, passphrase_file).await,
        Commands::Status => status::execute().await,
        Commands::Verify => verify::execute().await,
        Commands::BackupStore {
            output,
            passphrase_file,
        } => backup_store::execute(output, passphrase_file).await,
        Commands::Version => {
            version::execute();
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_register() {
        let cli = Cli::parse_from(["stroma", "register"]);

        match cli.command {
            Commands::Register {
                store_path,
                passphrase_file,
            } => {
                assert_eq!(store_path, None);
                assert_eq!(passphrase_file, None);
            }
            _ => panic!("Expected Register command"),
        }
    }

    #[test]
    fn test_cli_parse_link_device() {
        let cli = Cli::parse_from([
            "stroma",
            "link-device",
            "--device-name",
            "Test Bot",
            "--store-path",
            "/tmp/store",
        ]);

        match cli.command {
            Commands::LinkDevice {
                device_name,
                store_path,
                passphrase_file,
                ..
            } => {
                assert_eq!(device_name, "Test Bot");
                assert_eq!(store_path, Some("/tmp/store".to_string()));
                assert_eq!(passphrase_file, None);
            }
            _ => panic!("Expected LinkDevice command"),
        }
    }

    #[test]
    fn test_cli_parse_run() {
        let cli = Cli::parse_from(["stroma", "run", "--config", "/etc/stroma/config.toml"]);

        match cli.command {
            Commands::Run {
                config,
                bootstrap_contact,
                passphrase_file,
            } => {
                assert_eq!(config, "/etc/stroma/config.toml");
                assert_eq!(bootstrap_contact, None);
                assert_eq!(passphrase_file, None);
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_cli_parse_status() {
        let cli = Cli::parse_from(["stroma", "status"]);
        matches!(cli.command, Commands::Status);
    }

    #[test]
    fn test_cli_parse_verify() {
        let cli = Cli::parse_from(["stroma", "verify"]);
        matches!(cli.command, Commands::Verify);
    }

    #[test]
    fn test_cli_parse_backup_store() {
        let cli = Cli::parse_from(["stroma", "backup-store", "--output", "/tmp/backup.tar.gz"]);

        match cli.command {
            Commands::BackupStore {
                output,
                passphrase_file,
            } => {
                assert_eq!(output, "/tmp/backup.tar.gz");
                assert_eq!(passphrase_file, None);
            }
            _ => panic!("Expected BackupStore command"),
        }
    }

    #[test]
    fn test_cli_parse_version() {
        let cli = Cli::parse_from(["stroma", "version"]);
        matches!(cli.command, Commands::Version);
    }
}
