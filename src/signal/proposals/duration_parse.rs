//! Duration parsing for Signal group settings
//!
//! Supports human-readable durations like "1 hour", "7 days", etc.
//! Used for disappearing messages timer in /propose signal commands.

/// Parse human-readable duration to seconds.
///
/// Supports:
/// - "off" or "0" → 0 seconds
/// - Human-readable formats via humantime crate (e.g., "1 hour", "7 days", "1h", "7d")
///
/// # Arguments
/// * `input` - Duration string (e.g., "1 hour", "7d", "off")
///
/// # Returns
/// * `Ok(u32)` - Duration in seconds
/// * `Err(String)` - Parse error message
///
/// # Examples
/// ```
/// use stroma::signal::proposals::duration_parse::parse_duration_to_secs;
///
/// assert_eq!(parse_duration_to_secs("off").unwrap(), 0);
/// assert_eq!(parse_duration_to_secs("0").unwrap(), 0);
/// assert_eq!(parse_duration_to_secs("1 hour").unwrap(), 3600);
/// assert_eq!(parse_duration_to_secs("7 days").unwrap(), 604800);
/// assert_eq!(parse_duration_to_secs("1h").unwrap(), 3600);
/// assert_eq!(parse_duration_to_secs("7d").unwrap(), 604800);
/// ```
pub fn parse_duration_to_secs(input: &str) -> Result<u32, String> {
    // Handle "off" or "0" as special case (disable timer)
    if input == "off" || input == "0" {
        return Ok(0);
    }

    // Use humantime to parse flexible duration formats
    humantime::parse_duration(input)
        .map(|d| d.as_secs() as u32)
        .map_err(|e| format!("Invalid duration '{}': {}", input, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_off() {
        assert_eq!(parse_duration_to_secs("off").unwrap(), 0);
        assert_eq!(parse_duration_to_secs("0").unwrap(), 0);
    }

    #[test]
    fn test_parse_hours() {
        assert_eq!(parse_duration_to_secs("1h").unwrap(), 3600);
        assert_eq!(parse_duration_to_secs("1 hour").unwrap(), 3600);
        assert_eq!(parse_duration_to_secs("24h").unwrap(), 86400);
        assert_eq!(parse_duration_to_secs("24 hours").unwrap(), 86400);
    }

    #[test]
    fn test_parse_days() {
        assert_eq!(parse_duration_to_secs("1d").unwrap(), 86400);
        assert_eq!(parse_duration_to_secs("1 day").unwrap(), 86400);
        assert_eq!(parse_duration_to_secs("7d").unwrap(), 604800);
        assert_eq!(parse_duration_to_secs("7 days").unwrap(), 604800);
    }

    #[test]
    fn test_parse_common_signal_timers() {
        // Signal common disappearing messages timers
        assert_eq!(parse_duration_to_secs("1 hour").unwrap(), 3600);
        assert_eq!(parse_duration_to_secs("1 day").unwrap(), 86400);
        assert_eq!(parse_duration_to_secs("7 days").unwrap(), 604800);
        assert_eq!(parse_duration_to_secs("14 days").unwrap(), 1209600);
        assert_eq!(parse_duration_to_secs("30 days").unwrap(), 2592000);
        assert_eq!(parse_duration_to_secs("90 days").unwrap(), 7776000);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_duration_to_secs("invalid").is_err());
        assert!(parse_duration_to_secs("").is_err());
        assert!(parse_duration_to_secs("-5h").is_err());
    }
}
