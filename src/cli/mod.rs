use clap::{Parser, Subcommand};

pub mod backup_store;
pub mod config;
pub mod link_device;
pub mod passphrase;
pub mod register;
pub mod run;
pub mod status;
pub mod unregister;
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
    /// Register a new Stroma bot as primary device with Signal
    Register {
        /// Phone number in E.164 format (e.g., +16137827274)
        #[arg(long)]
        phone: String,

        /// Path to Signal protocol store (optional, uses default if not specified)
        #[arg(long)]
        store_path: Option<String>,

        /// Signal servers to use (production or staging)
        #[arg(long, default_value = "production")]
        servers: String,

        /// Use voice call instead of SMS for verification code
        #[arg(long)]
        voice: bool,

        /// Captcha token (required if previous attempt required captcha)
        #[arg(long)]
        captcha: Option<String>,

        /// Force re-registration even if already registered
        #[arg(long)]
        force: bool,

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
        /// Path to config file (default: adjacent to store at ~/.local/share/stroma/config.toml)
        #[arg(long)]
        config: Option<String>,

        /// Path to Signal protocol store (optional, uses default if not specified)
        #[arg(long)]
        store_path: Option<String>,

        /// Optional Signal contact to prompt for bootstrap initiation
        #[arg(long)]
        bootstrap_contact: Option<String>,

        /// Path to file containing passphrase (container-native)
        /// Default: ~/.local/share/stroma/passphrase.txt (adjacent to store)
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

    /// Unregister bot and clean up local data
    Unregister {
        /// Path to Signal protocol store (optional, uses default if not specified)
        #[arg(long)]
        store_path: Option<String>,

        /// Path to file containing passphrase (container-native)
        #[arg(long)]
        passphrase_file: Option<String>,

        /// Delete account from Signal servers (primary device only)
        /// Without this flag, only local data is removed
        #[arg(long)]
        delete_account: bool,

        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },
}

pub async fn execute(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Register {
            phone,
            store_path,
            servers,
            voice,
            captcha,
            force,
            passphrase_file,
        } => {
            register::execute(
                phone,
                store_path,
                servers,
                voice,
                captcha,
                force,
                passphrase_file,
            )
            .await
        }
        Commands::LinkDevice {
            device_name,
            store_path,
            servers,
            passphrase_file,
        } => link_device::execute(device_name, store_path, servers, passphrase_file).await,
        Commands::Run {
            config,
            store_path,
            bootstrap_contact,
            passphrase_file,
        } => run::execute(config, store_path, bootstrap_contact, passphrase_file).await,
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
        Commands::Unregister {
            store_path,
            passphrase_file,
            delete_account,
            yes,
        } => unregister::execute(store_path, passphrase_file, delete_account, yes).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_register() {
        let cli = Cli::parse_from(["stroma", "register", "--phone", "+16137827274"]);

        match cli.command {
            Commands::Register {
                phone,
                store_path,
                servers,
                voice,
                captcha,
                force,
                passphrase_file,
            } => {
                assert_eq!(phone, "+16137827274");
                assert_eq!(store_path, None);
                assert_eq!(servers, "production"); // default
                assert!(!voice);
                assert!(captcha.is_none());
                assert!(!force);
                assert_eq!(passphrase_file, None);
            }
            _ => panic!("Expected Register command"),
        }
    }

    #[test]
    fn test_cli_parse_register_with_all_options() {
        let cli = Cli::parse_from([
            "stroma",
            "register",
            "--phone",
            "+447911123456",
            "--store-path",
            "/tmp/store",
            "--servers",
            "staging",
            "--voice",
            "--captcha",
            "captcha-token-123",
            "--force",
            "--passphrase-file",
            "/tmp/key",
        ]);

        match cli.command {
            Commands::Register {
                phone,
                store_path,
                servers,
                voice,
                captcha,
                force,
                passphrase_file,
            } => {
                assert_eq!(phone, "+447911123456");
                assert_eq!(store_path, Some("/tmp/store".to_string()));
                assert_eq!(servers, "staging");
                assert!(voice);
                assert_eq!(captcha, Some("captcha-token-123".to_string()));
                assert!(force);
                assert_eq!(passphrase_file, Some("/tmp/key".to_string()));
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
                store_path,
                bootstrap_contact,
                passphrase_file,
            } => {
                assert_eq!(config, Some("/etc/stroma/config.toml".to_string()));
                assert_eq!(store_path, None);
                assert_eq!(bootstrap_contact, None);
                assert_eq!(passphrase_file, None);
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_cli_parse_run_defaults() {
        // Test that run works with no arguments (uses defaults)
        let cli = Cli::parse_from(["stroma", "run"]);

        match cli.command {
            Commands::Run {
                config,
                store_path,
                bootstrap_contact,
                passphrase_file,
            } => {
                assert_eq!(config, None);
                assert_eq!(store_path, None);
                assert_eq!(bootstrap_contact, None);
                assert_eq!(passphrase_file, None);
            }
            _ => panic!("Expected Run command"),
        }
    }

    #[test]
    fn test_cli_parse_run_with_store_path() {
        let cli = Cli::parse_from(["stroma", "run", "--store-path", "/custom/store"]);

        match cli.command {
            Commands::Run {
                config,
                store_path,
                bootstrap_contact,
                passphrase_file,
            } => {
                assert_eq!(config, None);
                assert_eq!(store_path, Some("/custom/store".to_string()));
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

    #[test]
    fn test_cli_parse_unregister() {
        let cli = Cli::parse_from(["stroma", "unregister"]);

        match cli.command {
            Commands::Unregister {
                store_path,
                passphrase_file,
                delete_account,
                yes,
            } => {
                assert_eq!(store_path, None);
                assert_eq!(passphrase_file, None);
                assert!(!delete_account);
                assert!(!yes);
            }
            _ => panic!("Expected Unregister command"),
        }
    }

    #[test]
    fn test_cli_parse_unregister_with_delete_account() {
        let cli = Cli::parse_from(["stroma", "unregister", "--delete-account", "--yes"]);

        match cli.command {
            Commands::Unregister {
                store_path,
                passphrase_file,
                delete_account,
                yes,
            } => {
                assert_eq!(store_path, None);
                assert_eq!(passphrase_file, None);
                assert!(delete_account);
                assert!(yes);
            }
            _ => panic!("Expected Unregister command"),
        }
    }

    #[test]
    fn test_cli_parse_unregister_with_store_path() {
        let cli = Cli::parse_from([
            "stroma",
            "unregister",
            "--store-path",
            "/tmp/my-store",
            "--passphrase-file",
            "/tmp/key",
        ]);

        match cli.command {
            Commands::Unregister {
                store_path,
                passphrase_file,
                delete_account,
                yes,
            } => {
                assert_eq!(store_path, Some("/tmp/my-store".to_string()));
                assert_eq!(passphrase_file, Some("/tmp/key".to_string()));
                assert!(!delete_account);
                assert!(!yes);
            }
            _ => panic!("Expected Unregister command"),
        }
    }
}
