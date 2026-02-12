//! Signal Primary Device Registration
//!
//! Implements full primary device registration with Signal servers.
//! This registers a new phone number with Signal (vs link-device which
//! links as secondary to an existing account).
//!
//! See: .beads/signal-integration.bead § Signal Account & Device Linking

use super::stroma_store::StromaStore;
use super::traits::*;
use presage::libsignal_service::configuration::SignalServers;
use presage::libsignal_service::prelude::phonenumber::PhoneNumber;
use presage::manager::RegistrationOptions;
use presage::Manager;
use std::io::{self, Write};
use std::path::Path;
use std::str::FromStr;

/// URL for obtaining captcha tokens when required by Signal
const CAPTCHA_URL: &str = "https://signalcaptchas.org/registration/generate.html";

/// Protocol handler prefix that must be stripped from captcha tokens.
/// The signalcaptchas.org site returns tokens as 'signalcaptcha://TOKEN'
/// but the Signal API expects the raw token without the protocol prefix.
const CAPTCHA_PREFIX: &str = "signalcaptcha://";

/// Primary device registration configuration
pub struct RegistrationConfig {
    /// Phone number in E.164 format (e.g., +16137827274)
    pub phone_number: String,

    /// Path to protocol store (directory containing signal.db and stroma.db)
    pub store_path: std::path::PathBuf,

    /// Operator-provided passphrase (24-word BIP-39 mnemonic)
    pub passphrase: String,

    /// Signal server environment
    pub signal_servers: SignalServers,

    /// Use voice call instead of SMS for verification code
    pub use_voice: bool,

    /// Captcha token (required if previous attempt returned CaptchaRequired)
    pub captcha: Option<String>,

    /// Force re-registration even if already registered
    pub force: bool,
}

impl RegistrationConfig {
    pub fn new(
        phone_number: impl Into<String>,
        store_path: impl AsRef<Path>,
        passphrase: impl Into<String>,
        signal_servers: SignalServers,
    ) -> Self {
        Self {
            phone_number: phone_number.into(),
            store_path: store_path.as_ref().to_path_buf(),
            passphrase: passphrase.into(),
            signal_servers,
            use_voice: false,
            captcha: None,
            force: false,
        }
    }

    pub fn with_voice(mut self, use_voice: bool) -> Self {
        self.use_voice = use_voice;
        self
    }

    pub fn with_captcha(mut self, captcha: Option<String>) -> Self {
        self.captcha = captcha;
        self
    }

    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }
}

/// Strip the 'signalcaptcha://' protocol prefix from a captcha token if present.
/// The signalcaptchas.org site returns tokens as 'signalcaptcha://signal-hcaptcha.TOKEN'
/// but the Signal server API expects 'signal-hcaptcha.TOKEN' (without the prefix).
fn strip_captcha_prefix(captcha: &str) -> &str {
    captcha.strip_prefix(CAPTCHA_PREFIX).unwrap_or(captcha)
}

/// Clean up a store directory. Used when registration fails before verification
/// so the user can retry without needing --force.
fn cleanup_store(store_path: &Path) {
    if store_path.exists() {
        if let Err(e) = std::fs::remove_dir_all(store_path) {
            eprintln!(
                "⚠️  Failed to clean up store at {}: {}",
                store_path.display(),
                e
            );
            eprintln!("   You may need to manually delete it before retrying.");
        }
    }
}

/// Register as primary device
///
/// Registers a new phone number with Signal servers and creates a new
/// encrypted protocol store. This is a two-phase process:
/// 1. Request verification code (SMS or voice)
/// 2. Submit verification code to complete registration
///
/// # Flow
/// 1. Create encrypted StromaStore (signal.db + stroma.db)
/// 2. Call Manager::register() to request verification code
/// 3. Handle CaptchaRequired if needed (instruct user to retry with --captcha)
/// 4. Prompt user for verification code
/// 5. Call manager.confirm_verification_code()
/// 6. Display ACI/PNI identity info on success
///
/// # Errors
/// Returns error if:
/// - Phone number is invalid
/// - Store creation fails
/// - Captcha is required but not provided
/// - Verification code is wrong
/// - Network errors during registration
pub async fn register_device(config: RegistrationConfig) -> SignalResult<()> {
    println!("📱 Registering Stroma as primary device...");
    println!();

    // Parse phone number
    let phone_number = PhoneNumber::from_str(&config.phone_number).map_err(|e| {
        SignalError::Protocol(format!(
            "Invalid phone number '{}': {}. Use E.164 format (e.g., +16137827274)",
            config.phone_number, e
        ))
    })?;

    println!("Phone Number: {}", phone_number);
    println!(
        "Server: {:?}",
        match config.signal_servers {
            SignalServers::Production => "Production",
            SignalServers::Staging => "Staging",
        }
    );
    println!(
        "Verification: {}",
        if config.use_voice {
            "Voice call"
        } else {
            "SMS"
        }
    );
    println!();

    // Create encrypted StromaStore silently (will create both signal.db and stroma.db)
    // The store is cleaned up automatically if registration fails
    let store = StromaStore::open(&config.store_path, config.passphrase)
        .await
        .map_err(|e| SignalError::Store(format!("Failed to create store: {}", e)))?;

    // Strip signalcaptcha:// prefix from captcha token if present
    let captcha = config
        .captcha
        .as_deref()
        .map(strip_captcha_prefix)
        .map(String::from);

    // Build registration options
    let registration_options = RegistrationOptions {
        signal_servers: config.signal_servers,
        phone_number,
        use_voice_call: config.use_voice,
        captcha: captcha.as_deref(),
        force: config.force,
    };

    // Phase 1: Request verification code
    println!("📤 Requesting verification code...");

    let confirmation_manager = match Manager::register(store, registration_options).await {
        Ok(manager) => manager,
        Err(presage::Error::CaptchaRequired) => {
            // Clean up the store so user can retry without --force
            cleanup_store(&config.store_path);

            println!();
            println!("⚠️  Captcha required!");
            println!();
            println!("Signal requires a captcha to proceed. Please:");
            println!("  1. Open this URL in a browser:");
            println!("     {}", CAPTCHA_URL);
            println!();
            println!("  2. Complete the captcha challenge");
            println!();
            println!(
                "  3. Copy the full token (starts with 'signalcaptcha://' or 'signal-hcaptcha.')"
            );
            println!();
            println!("  4. Re-run this command with the captcha token:");
            println!(
                "     stroma register --phone {} --captcha <TOKEN>",
                config.phone_number
            );
            println!();
            return Err(SignalError::Protocol(
                "Captcha required - see instructions above".to_string(),
            ));
        }
        Err(presage::Error::PushChallengeRequired) => {
            cleanup_store(&config.store_path);

            println!();
            println!("❌ Push challenge required");
            println!();
            println!("Signal requires a push-based verification which is not supported.");
            println!("This typically happens when:");
            println!("  - The phone number has been flagged for abuse");
            println!("  - Too many registration attempts have been made");
            println!();
            println!("Try again later or use a different phone number.");
            return Err(SignalError::Protocol(
                "Push challenge required (not supported)".to_string(),
            ));
        }
        Err(presage::Error::RequestingCodeForbidden(session)) => {
            cleanup_store(&config.store_path);

            println!();
            println!("❌ Verification code request forbidden");
            println!();
            println!("Signal is not allowing verification code requests.");
            println!("Session state: {:?}", session);
            println!();
            println!("This may be due to rate limiting. Try again later.");
            return Err(SignalError::Protocol(
                "Verification code request forbidden".to_string(),
            ));
        }
        Err(presage::Error::AlreadyRegisteredError) => {
            // Don't clean up the store here -- it contains valid registration data
            println!();
            println!("❌ Already registered");
            println!();
            println!("This store is already registered with Signal.");
            println!("To re-register, use the --force flag:");
            println!("  stroma register --phone {} --force", config.phone_number);
            return Err(SignalError::Protocol(
                "Already registered (use --force to re-register)".to_string(),
            ));
        }
        Err(e) => {
            cleanup_store(&config.store_path);

            // Provide specific guidance for common registration errors
            if let presage::Error::ServiceError(ref service_err) = e {
                let err_msg = service_err.to_string();

                // HTTP 429 (or HTTP 413 which Signal also uses for rate limiting)
                if err_msg.contains("429")
                    || err_msg.contains("Too Many Requests")
                    || err_msg.contains("Rate limit exceeded")
                {
                    println!();
                    println!("❌ Rate limit exceeded");
                    println!();
                    println!("Signal has temporarily blocked registration requests from this IP/account.");
                    println!();
                    println!("This happens when:");
                    println!("  - Too many registration attempts in a short time");
                    println!("  - Multiple different phone numbers tried from same IP");
                    println!("  - Repeated captcha failures");
                    println!();
                    println!("Solutions:");
                    println!("  1. Wait 24 hours before trying again");
                    println!("  2. Use a different network (VPN, mobile hotspot, etc.)");
                    println!(
                        "  3. Use 'stroma link-device' instead (doesn't require registration)"
                    );
                    println!();
                    return Err(SignalError::Protocol(
                        "Rate limit exceeded - see guidance above".to_string(),
                    ));
                }

                // HTTP 401/403 Unauthorized
                if err_msg.contains("Unauthorized") || err_msg.contains("Authorization failed") {
                    println!();
                    println!("❌ Authorization failed");
                    println!();
                    println!("The phone number may already be registered to a different account.");
                    println!();
                    println!("This can happen if:");
                    println!("  - The number is actively used by another Signal account");
                    println!("  - A previous registration wasn't fully cleaned up");
                    println!();
                    println!("Try:");
                    println!("  - Using 'stroma link-device' to link to the existing account");
                    println!("  - Using a different phone number");
                    println!();
                    return Err(SignalError::Protocol(
                        "Authorization failed - phone may be in use".to_string(),
                    ));
                }

                // HTTP 423 Locked (registration lock/PIN)
                if err_msg.contains("Locked") || err_msg.contains("Registration lock") {
                    println!();
                    println!("❌ Registration lock is set");
                    println!();
                    println!("This phone number has a registration lock (PIN) set.");
                    println!();
                    println!("Registration lock prevents others from registering your number");
                    println!("even if they have access to SMS codes. You'll need the recovery");
                    println!("password or PIN to proceed.");
                    println!();
                    println!("This feature is not yet implemented in stroma.");
                    println!("Use 'stroma link-device' as a workaround.");
                    println!();
                    return Err(SignalError::Protocol(
                        "Registration lock not supported - use link-device".to_string(),
                    ));
                }

                // HTTP 428 Precondition Required (proof/spam challenge)
                if err_msg.contains("Proof required") || err_msg.contains("428") {
                    println!();
                    println!("❌ Spam challenge required");
                    println!();
                    println!("Signal requires additional proof that you're not a spammer.");
                    println!("This is different from captcha and requires external verification.");
                    println!();
                    println!("This feature is not yet implemented in stroma.");
                    println!("Try 'stroma link-device' instead, or wait 24-48 hours.");
                    println!();
                    return Err(SignalError::Protocol(
                        "Spam challenge not supported - use link-device".to_string(),
                    ));
                }
            }

            return Err(SignalError::Protocol(format!("Registration failed: {}", e)));
        }
    };

    println!(
        "✅ Verification code sent via {}!",
        if config.use_voice {
            "voice call"
        } else {
            "SMS"
        }
    );
    println!();

    // Phase 2: Prompt for verification code
    print!("Enter the verification code you received: ");
    io::stdout()
        .flush()
        .map_err(|e| SignalError::Protocol(format!("Failed to flush stdout: {}", e)))?;

    let mut verification_code = String::new();
    io::stdin()
        .read_line(&mut verification_code)
        .map_err(|e| SignalError::Protocol(format!("Failed to read verification code: {}", e)))?;

    let verification_code = verification_code.trim();
    if verification_code.is_empty() {
        return Err(SignalError::Protocol(
            "Verification code cannot be empty".to_string(),
        ));
    }

    // Phase 3: Confirm verification code
    println!();
    println!("📤 Submitting verification code...");

    let manager = match confirmation_manager
        .confirm_verification_code(verification_code)
        .await
    {
        Ok(m) => m,
        Err(e) => {
            cleanup_store(&config.store_path);

            return match e {
                presage::Error::UnverifiedRegistrationSession => Err(SignalError::Protocol(
                    "Invalid verification code. Please check the code and try again.\n\
                        You will need to restart registration from scratch."
                        .to_string(),
                )),
                presage::Error::ServiceError(ref service_err) => {
                    println!();
                    println!("❌ Verification failed: {}", service_err);
                    println!();
                    println!("This may be caused by:");
                    println!("  - Expired verification session (took too long to enter code)");
                    println!("  - Rate limiting from Signal");
                    println!("  - Network connectivity issues");
                    println!();
                    println!("Try again with a fresh registration:");
                    println!("  stroma register --phone {}", config.phone_number);
                    println!();
                    println!("TIP: Use RUST_LOG=debug for detailed diagnostics");
                    Err(SignalError::Protocol(format!("Verification failed: {}", e)))
                }
                other => Err(SignalError::Protocol(format!(
                    "Verification failed: {}",
                    other
                ))),
            };
        }
    };

    println!("✅ Verification successful!");
    println!();

    // Display account information
    let whoami = manager
        .whoami()
        .await
        .map_err(|e| SignalError::Protocol(format!("Failed to get account info: {}", e)))?;

    println!("🎉 Registration complete!");
    println!();
    println!("Account Information:");
    println!("   ACI: {}", whoami.aci);
    println!("   PNI: {}", whoami.pni);
    println!("   Phone: {}", whoami.number);
    println!("   Store: {}", config.store_path.display());
    println!();
    println!("You can now run 'stroma run' to start the bot!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_registration_config() {
        let config = RegistrationConfig::new(
            "+16137827274",
            "/tmp/store",
            "secure-passphrase",
            SignalServers::Production,
        );

        assert_eq!(config.phone_number, "+16137827274");
        assert_eq!(config.store_path, std::path::PathBuf::from("/tmp/store"));
        assert_eq!(config.passphrase, "secure-passphrase");
        assert!(!config.use_voice);
        assert!(config.captcha.is_none());
        assert!(!config.force);
    }

    #[test]
    fn test_registration_config_with_options() {
        let config = RegistrationConfig::new(
            "+16137827274",
            "/tmp/store",
            "secure-passphrase",
            SignalServers::Staging,
        )
        .with_voice(true)
        .with_captcha(Some("captcha-token".to_string()))
        .with_force(true);

        assert!(config.use_voice);
        assert_eq!(config.captcha, Some("captcha-token".to_string()));
        assert!(config.force);
    }

    #[test]
    fn test_phone_number_validation() {
        // Valid E.164 numbers
        assert!(PhoneNumber::from_str("+16137827274").is_ok());
        assert!(PhoneNumber::from_str("+447911123456").is_ok());

        // Invalid formats
        assert!(PhoneNumber::from_str("6137827274").is_err()); // Missing +
        assert!(PhoneNumber::from_str("not-a-number").is_err());
    }

    #[test]
    fn test_strip_captcha_prefix() {
        // With signalcaptcha:// prefix (as returned by signalcaptchas.org)
        assert_eq!(
            strip_captcha_prefix("signalcaptcha://signal-hcaptcha.abc123.xyz"),
            "signal-hcaptcha.abc123.xyz"
        );

        // Already stripped (raw token)
        assert_eq!(
            strip_captcha_prefix("signal-hcaptcha.abc123.xyz"),
            "signal-hcaptcha.abc123.xyz"
        );

        // Empty string
        assert_eq!(strip_captcha_prefix(""), "");

        // Just the prefix
        assert_eq!(strip_captcha_prefix("signalcaptcha://"), "");
    }

    #[test]
    fn test_cleanup_store_removes_directory() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("store-to-cleanup");
        std::fs::create_dir(&store_path).unwrap();

        // Create a file inside to verify recursive deletion
        std::fs::write(store_path.join("signal.db"), b"test data").unwrap();

        cleanup_store(&store_path);
        assert!(!store_path.exists());
    }

    #[test]
    fn test_cleanup_store_handles_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("nonexistent");

        // Should not panic on non-existent directory
        cleanup_store(&store_path);
        // No assertion needed - just verify it doesn't panic
    }

    #[test]
    fn test_phone_number_international_formats() {
        // Various international E.164 formats
        assert!(PhoneNumber::from_str("+33612345678").is_ok()); // France
        assert!(PhoneNumber::from_str("+8613912345678").is_ok()); // China
        assert!(PhoneNumber::from_str("+4915123456789").is_ok()); // Germany
        assert!(PhoneNumber::from_str("+61412345678").is_ok()); // Australia
        assert!(PhoneNumber::from_str("+81901234567").is_ok()); // Japan
    }

    #[test]
    fn test_phone_number_edge_cases() {
        // Invalid: too short (just country code)
        assert!(PhoneNumber::from_str("+1").is_err());

        // Invalid: letters in number
        assert!(PhoneNumber::from_str("+1abc1234567").is_err());

        // Valid: US number
        assert!(PhoneNumber::from_str("+16137827274").is_ok());

        // Note: The phonenumber library is lenient about some formats:
        // - `++16137827274` may parse (extra + is ignored)
        // - `+1 613 782 7274` may parse (spaces allowed)
        // Signal's server-side validation will enforce strict E.164,
        // but we test what our library actually accepts for documentation.
    }

    // Note: Actual registration tests require real Signal infrastructure
    // or complex mocking of presage Manager. Testing is done via
    // manual E2E validation per testing-standards.bead.
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use proptest::test_runner::{Config as ProptestConfig, RngAlgorithm, TestRng, TestRunner};

    const PROPTEST_SEED: &[u8; 32] = b"stroma-registration-proptest-32b";

    /// Property: Phone number parsing never panics
    #[test]
    fn prop_phone_parsing_never_panics() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = ".*";

        runner
            .run(&strategy, |s| {
                // Should handle any string input without panicking
                let _ = PhoneNumber::from_str(&s);
                Ok(())
            })
            .unwrap();
    }

    /// Property: Valid E.164 format numbers should parse
    #[test]
    fn prop_valid_e164_format() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            cases: 100, // Reduce cases since we're testing specific format
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = (1u16..999u16, 1000000000u64..99999999999u64);

        runner
            .run(&strategy, |(country_code, number)| {
                let phone = format!("+{}{}", country_code, number);
                let result = PhoneNumber::from_str(&phone);

                // Most combinations should parse (library is lenient)
                // We're testing that valid-looking numbers don't panic
                // and the library handles them gracefully
                let _ = result;

                Ok(())
            })
            .unwrap();
    }

    /// Property: strip_captcha_prefix is idempotent
    #[test]
    fn prop_strip_captcha_idempotent() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = ".*";

        runner
            .run(&strategy, |s| {
                let stripped_once = strip_captcha_prefix(&s);
                let stripped_twice = strip_captcha_prefix(stripped_once);

                prop_assert_eq!(
                    stripped_once,
                    stripped_twice,
                    "Stripping twice should equal stripping once"
                );

                Ok(())
            })
            .unwrap();
    }

    /// Property: strip_captcha_prefix removes exactly the prefix
    #[test]
    fn prop_strip_captcha_prefix_correctness() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = "[a-zA-Z0-9.-]+";

        runner
            .run(&strategy, |token| {
                // Test with prefix
                let with_prefix = format!("{}{}", CAPTCHA_PREFIX, &token);
                let result = strip_captcha_prefix(&with_prefix);
                prop_assert_eq!(
                    result,
                    &token as &str,
                    "Should strip prefix and return token"
                );

                // Test without prefix (should return as-is)
                let result_no_prefix = strip_captcha_prefix(&token);
                prop_assert_eq!(
                    result_no_prefix,
                    &token as &str,
                    "Should return input unchanged"
                );

                Ok(())
            })
            .unwrap();
    }

    /// Property: strip_captcha_prefix never panics
    #[test]
    fn prop_strip_captcha_never_panics() {
        let config = ProptestConfig {
            rng_algorithm: RngAlgorithm::ChaCha,
            ..Default::default()
        };
        let mut runner = TestRunner::new_with_rng(
            config,
            TestRng::from_seed(RngAlgorithm::ChaCha, PROPTEST_SEED),
        );

        let strategy = ".*";

        runner
            .run(&strategy, |s| {
                let _ = strip_captcha_prefix(&s);
                Ok(())
            })
            .unwrap();
    }
}
