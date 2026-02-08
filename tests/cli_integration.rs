// Integration tests for CLI commands
// These tests verify that the CLI parsing and command execution work correctly
// without requiring the full Signal/Freenet stack to be operational.

use std::io::Write;
use std::process::Command;
use tempfile::{NamedTempFile, TempDir};

fn get_binary_path() -> String {
    // The binary path will be in target/debug/, target/release/, or target/llvm-cov-target/debug/
    let binary_name = if cfg!(windows) {
        "stroma.exe"
    } else {
        "stroma"
    };

    // Try to find the binary (check llvm-cov location first since that's where it'll be during coverage runs)
    let llvm_cov_path = format!("target/llvm-cov-target/debug/{}", binary_name);
    let debug_path = format!("target/debug/{}", binary_name);
    let release_path = format!("target/release/{}", binary_name);

    if std::path::Path::new(&llvm_cov_path).exists() {
        llvm_cov_path
    } else if std::path::Path::new(&debug_path).exists() {
        debug_path
    } else if std::path::Path::new(&release_path).exists() {
        release_path
    } else {
        panic!("Binary not found. Run `cargo build` first.");
    }
}

#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_cli_help() {
    let output = Command::new(get_binary_path())
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Operator CLI for Stroma"));
    assert!(stdout.contains("link-device"));
    assert!(stdout.contains("run"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("verify"));
    assert!(stdout.contains("backup-store"));
    assert!(stdout.contains("version"));
}

#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_cli_version() {
    let output = Command::new(get_binary_path())
        .arg("version")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("stroma"));
    assert!(stdout.contains("Operator CLI"));
}

#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_cli_link_device_requires_device_name() {
    let output = Command::new(get_binary_path())
        .arg("link-device")
        .output()
        .expect("Failed to execute command");

    // Should fail because --device-name is required
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("device-name") || stderr.contains("required"));
}

#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_cli_link_device_with_device_name() {
    let output = Command::new(get_binary_path())
        .arg("link-device")
        .arg("--device-name")
        .arg("Test Bot")
        .output()
        .expect("Failed to execute command");

    // Command should execute (even if Signal integration not complete)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Linking") || stdout.contains("Device Name"));
}

#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_cli_run_requires_config() {
    let output = Command::new(get_binary_path())
        .arg("run")
        .output()
        .expect("Failed to execute command");

    // Should fail because --config is required
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("config") || stderr.contains("required"));
}

#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_cli_run_with_missing_config() {
    let output = Command::new(get_binary_path())
        .arg("run")
        .arg("--config")
        .arg("/nonexistent/config.toml")
        .output()
        .expect("Failed to execute command");

    // Should fail because config file doesn't exist
    assert!(!output.status.success());
}

#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_cli_run_with_valid_config() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "[signal]").unwrap();
    writeln!(temp_file, "store_path = \"/tmp/store\"").unwrap();

    let output = Command::new(get_binary_path())
        .arg("run")
        .arg("--config")
        .arg(temp_file.path())
        .output()
        .expect("Failed to execute command");

    // Command should execute (even if bot service not complete)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Starting") || stdout.contains("Config"));
}

#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_cli_status() {
    let output = Command::new(get_binary_path())
        .arg("status")
        .output()
        .expect("Failed to execute command");

    // Command should execute
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Status") || stdout.contains("Bot"));
}

#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_cli_verify() {
    let output = Command::new(get_binary_path())
        .arg("verify")
        .output()
        .expect("Failed to execute command");

    // Command should execute and show verification results
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Verifying") || stdout.contains("Binary"));
}

#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_cli_backup_store_requires_output() {
    let output = Command::new(get_binary_path())
        .arg("backup-store")
        .output()
        .expect("Failed to execute command");

    // Should fail because --output is required
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("output") || stderr.contains("required"));
}

#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_cli_backup_store_with_invalid_output_dir() {
    let output = Command::new(get_binary_path())
        .arg("backup-store")
        .arg("--output")
        .arg("/nonexistent/dir/backup.tar.gz")
        .output()
        .expect("Failed to execute command");

    // Should fail because output directory doesn't exist
    assert!(!output.status.success());
}

#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_cli_backup_store_with_valid_output() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("backup.tar.gz");

    let output = Command::new(get_binary_path())
        .arg("backup-store")
        .arg("--output")
        .arg(&output_path)
        .output()
        .expect("Failed to execute command");

    // Command should execute (even if actual backup not complete)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Backing up") || stdout.contains("Signal"));
}

// Test all commands show help text
#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_subcommand_help() {
    let commands = vec![
        "link-device",
        "run",
        "status",
        "verify",
        "backup-store",
        "version",
    ];

    for cmd in commands {
        let output = Command::new(get_binary_path())
            .arg(cmd)
            .arg("--help")
            .output()
            .unwrap_or_else(|_| panic!("Failed to execute {} --help", cmd));

        assert!(output.status.success(), "Command {} --help failed", cmd);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.is_empty(),
            "Command {} --help produced no output",
            cmd
        );
    }
}

// Test that invalid commands are rejected
#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_invalid_command() {
    let output = Command::new(get_binary_path())
        .arg("invalid-command")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid-command") || stderr.contains("unrecognized"));
}

// Test command variations and edge cases
#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_link_device_with_all_options() {
    let output = Command::new(get_binary_path())
        .arg("link-device")
        .arg("--device-name")
        .arg("Test Bot")
        .arg("--store-path")
        .arg("/tmp/test-store")
        .arg("--servers")
        .arg("staging")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Test Bot"));
    assert!(stdout.contains("/tmp/test-store") || stdout.contains("staging"));
}

#[test]
#[ignore] // Ignore until presage dependency is fixed
fn test_run_with_bootstrap_contact() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "[signal]").unwrap();

    let output = Command::new(get_binary_path())
        .arg("run")
        .arg("--config")
        .arg(temp_file.path())
        .arg("--bootstrap-contact")
        .arg("@alice")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("@alice") || stdout.contains("bootstrap"));
}
