//! Command parsing for /propose.
//!
//! Syntax:
//! - /propose config <key> <value> [--timeout <duration>]
//! - /propose stroma <key> <value> [--timeout <duration>]
//!
//! Duration format: Nh (hours), e.g., 48h, 72h
//! Min: 1h, Max: 168h (1 week)

use std::time::Duration;

/// Parsed /propose arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct ProposeArgs {
    pub subcommand: ProposalSubcommand,
    pub timeout: Option<Duration>,
}

/// Proposal subcommand types.
#[derive(Debug, Clone, PartialEq)]
pub enum ProposalSubcommand {
    /// Change group config (e.g., min_vouches, max_flags).
    Config { key: String, value: String },
    /// Change stroma config (app-level settings).
    Stroma { key: String, value: String },
}

/// Parse /propose command arguments.
///
/// Expected formats:
/// - /propose config min_vouches 3 --timeout 48h
/// - /propose stroma name "New Name"
///
/// Returns Ok(ProposeArgs) or Err(error message).
pub fn parse_propose_args(subcommand: &str, args: &[String]) -> Result<ProposeArgs, String> {
    // Parse subcommand
    let (proposal_type, remaining) = match subcommand {
        "config" => {
            if args.len() < 2 {
                return Err(
                    "Usage: /propose config <key> <value> [--timeout <duration>]".to_string(),
                );
            }
            let key = args[0].clone();
            let value = args[1].clone();
            let remaining = &args[2..];
            (ProposalSubcommand::Config { key, value }, remaining)
        }
        "stroma" => {
            if args.len() < 2 {
                return Err(
                    "Usage: /propose stroma <key> <value> [--timeout <duration>]".to_string(),
                );
            }
            let key = args[0].clone();
            let value = args[1].clone();
            let remaining = &args[2..];
            (ProposalSubcommand::Stroma { key, value }, remaining)
        }
        _ => {
            return Err(format!(
                "Unknown proposal type: {}. Use 'config' or 'stroma'.",
                subcommand
            ));
        }
    };

    // Parse optional --timeout flag
    let timeout = parse_timeout_flag(remaining)?;

    Ok(ProposeArgs {
        subcommand: proposal_type,
        timeout,
    })
}

/// Parse --timeout flag from remaining args.
///
/// Expected format: --timeout 48h
/// Min: 1h (3600s), Max: 168h (604800s)
fn parse_timeout_flag(args: &[String]) -> Result<Option<Duration>, String> {
    // Find --timeout flag
    let timeout_idx = args.iter().position(|a| a == "--timeout");

    if let Some(idx) = timeout_idx {
        if idx + 1 >= args.len() {
            return Err("--timeout flag requires a duration argument (e.g., 48h)".to_string());
        }

        let duration_str = &args[idx + 1];
        let duration = parse_duration(duration_str)?;

        // Validate bounds: min 1h, max 168h
        const MIN_TIMEOUT: Duration = Duration::from_secs(3600); // 1 hour
        const MAX_TIMEOUT: Duration = Duration::from_secs(604800); // 168 hours (1 week)

        if duration < MIN_TIMEOUT {
            return Err(format!(
                "Timeout must be at least 1h. Got: {}",
                duration_str
            ));
        }

        if duration > MAX_TIMEOUT {
            return Err(format!(
                "Timeout must be at most 168h (1 week). Got: {}",
                duration_str
            ));
        }

        Ok(Some(duration))
    } else {
        Ok(None)
    }
}

/// Parse duration string (e.g., "48h", "72h").
///
/// Supports hours only (e.g., "48h").
fn parse_duration(s: &str) -> Result<Duration, String> {
    if let Some(hours_str) = s.strip_suffix('h') {
        let hours: u64 = hours_str
            .parse()
            .map_err(|_| format!("Invalid hour value: {}", hours_str))?;
        Ok(Duration::from_secs(hours * 3600))
    } else {
        Err(format!("Duration must end with 'h' (hours). Got: {}", s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config_proposal() {
        let result = parse_propose_args("config", &["min_vouches".to_string(), "3".to_string()]);
        assert!(result.is_ok());
        let args = result.unwrap();
        assert!(matches!(args.subcommand, ProposalSubcommand::Config { .. }));
        assert_eq!(args.timeout, None);
    }

    #[test]
    fn test_parse_config_with_timeout() {
        let result = parse_propose_args(
            "config",
            &[
                "min_vouches".to_string(),
                "3".to_string(),
                "--timeout".to_string(),
                "48h".to_string(),
            ],
        );
        assert!(result.is_ok());
        let args = result.unwrap();
        assert_eq!(args.timeout, Some(Duration::from_secs(48 * 3600)));
    }

    #[test]
    fn test_parse_stroma_proposal() {
        let result = parse_propose_args("stroma", &["name".to_string(), "New Name".to_string()]);
        assert!(result.is_ok());
        let args = result.unwrap();
        assert!(matches!(args.subcommand, ProposalSubcommand::Stroma { .. }));
    }

    #[test]
    fn test_timeout_too_short() {
        let result = parse_propose_args(
            "config",
            &[
                "min_vouches".to_string(),
                "3".to_string(),
                "--timeout".to_string(),
                "0h".to_string(),
            ],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least 1h"));
    }

    #[test]
    fn test_timeout_too_long() {
        let result = parse_propose_args(
            "config",
            &[
                "min_vouches".to_string(),
                "3".to_string(),
                "--timeout".to_string(),
                "200h".to_string(),
            ],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at most 168h"));
    }

    #[test]
    fn test_unknown_subcommand() {
        let result = parse_propose_args("unknown", &["arg1".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown proposal type"));
    }

    #[test]
    fn test_missing_args() {
        let result = parse_propose_args("config", &["min_vouches".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Usage"));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property test: Timeout bounds validation
        /// All timeouts in valid range [1h, 168h] should be accepted
        /// All timeouts outside this range should be rejected
        #[test]
        fn prop_timeout_bounds_valid_range(
            hours in 1u64..=168,
        ) {
            let timeout_str = format!("{}h", hours);
            let result = parse_propose_args(
                "config",
                &[
                    "min_vouches".to_string(),
                    "3".to_string(),
                    "--timeout".to_string(),
                    timeout_str,
                ],
            );

            // All timeouts in [1h, 168h] should succeed
            prop_assert!(result.is_ok(), "Valid timeout {}h should be accepted", hours);

            if let Ok(args) = result {
                let expected_duration = Duration::from_secs(hours * 3600);
                prop_assert_eq!(
                    args.timeout,
                    Some(expected_duration),
                    "Timeout should be {} seconds",
                    hours * 3600
                );
            }
        }

        /// Property test: Timeout below minimum should fail
        /// Any timeout < 1h should be rejected with appropriate error
        #[test]
        fn prop_timeout_below_minimum(
            // Use small values to test below minimum (0h is edge case)
            _minutes in 0u64..60,
        ) {
            // Convert minutes to fractional hours, but parse_duration only handles hours
            // So we test with 0h which should always fail
            let result = parse_propose_args(
                "config",
                &[
                    "min_vouches".to_string(),
                    "3".to_string(),
                    "--timeout".to_string(),
                    "0h".to_string(),
                ],
            );

            // Should fail with "at least 1h" message
            prop_assert!(result.is_err(), "Timeout 0h should be rejected");
            let error_msg = result.unwrap_err();
            prop_assert!(
                error_msg.contains("at least 1h"),
                "Error should mention minimum of 1h, got: {}",
                error_msg
            );
        }

        /// Property test: Timeout above maximum should fail
        /// Any timeout > 168h should be rejected with appropriate error
        #[test]
        fn prop_timeout_above_maximum(
            extra_hours in 1u64..1000,
        ) {
            let invalid_hours = 168 + extra_hours;
            let timeout_str = format!("{}h", invalid_hours);
            let result = parse_propose_args(
                "config",
                &[
                    "min_vouches".to_string(),
                    "3".to_string(),
                    "--timeout".to_string(),
                    timeout_str.clone(),
                ],
            );

            // Should fail with "at most 168h" message
            prop_assert!(
                result.is_err(),
                "Timeout {} should be rejected",
                timeout_str
            );
            let error_msg = result.unwrap_err();
            prop_assert!(
                error_msg.contains("at most 168h"),
                "Error should mention maximum of 168h, got: {}",
                error_msg
            );
        }

        /// Property test: Timeout boundary validation consistency
        /// Verify that MIN_TIMEOUT (1h) and MAX_TIMEOUT (168h) are correctly enforced
        #[test]
        fn prop_timeout_boundary_consistency(
            timeout_hours in 0u64..200,
        ) {
            let timeout_str = format!("{}h", timeout_hours);
            let result = parse_propose_args(
                "config",
                &[
                    "min_vouches".to_string(),
                    "3".to_string(),
                    "--timeout".to_string(),
                    timeout_str,
                ],
            );

            if (1..=168).contains(&timeout_hours) {
                // Valid range: should succeed
                prop_assert!(
                    result.is_ok(),
                    "Timeout {}h is in valid range [1, 168] but was rejected",
                    timeout_hours
                );
            } else {
                // Invalid range: should fail
                prop_assert!(
                    result.is_err(),
                    "Timeout {}h is outside valid range [1, 168] but was accepted",
                    timeout_hours
                );
            }
        }
    }
}
