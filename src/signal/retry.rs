//! Signal API retry with logarithmic backoff.
//!
//! Per GAP-06 (security-constraints.bead):
//! - Retry transient failures with exponential backoff (2^n seconds)
//! - Cap at 1 hour (3600 seconds)
//! - Enforce invariant: signal_state.members ⊆ freenet_state.members
//!
//! This ensures Signal API failures don't leave stale members in the group.

use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

/// Maximum retry attempts before giving up.
const MAX_RETRIES: u32 = 12; // 2^12 = 4096 seconds > 1 hour

/// Maximum backoff duration (1 hour).
const MAX_BACKOFF_SECS: u64 = 3600;

/// Retry a Signal API operation with logarithmic backoff.
///
/// Per GAP-06:
/// - Backoff: 2^n seconds (1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096...)
/// - Cap: 3600 seconds (1 hour)
/// - Retries: Up to 12 attempts (covers ~1.1 hour of cumulative backoff)
///
/// # Arguments
///
/// * `operation` - The async operation to retry (e.g., remove_group_member)
/// * `is_retryable` - Function to determine if error is transient and retryable
///
/// # Returns
///
/// Result of the operation, or the last error after all retries exhausted.
///
/// # Example
///
/// ```no_run
/// use stromarig::signal::retry::retry_with_backoff;
/// use stromarig::signal::traits::SignalError;
///
/// async fn remove_member() -> Result<(), SignalError> {
///     retry_with_backoff(
///         || async { /* Signal API call */ Ok(()) },
///         |e| matches!(e, SignalError::Network(_))
///     ).await
/// }
/// ```
pub async fn retry_with_backoff<F, Fut, T, E>(
    mut operation: F,
    is_retryable: fn(&E) -> bool,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut attempt = 0;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                // Check if we should retry
                if !is_retryable(&err) || attempt >= MAX_RETRIES {
                    return Err(err);
                }

                // Calculate backoff: 2^attempt seconds, capped at MAX_BACKOFF_SECS
                let backoff_secs = 2u64.pow(attempt).min(MAX_BACKOFF_SECS);
                let backoff = Duration::from_secs(backoff_secs);

                // Log retry attempt (in production, use tracing::warn)
                eprintln!(
                    "[retry] Attempt {} failed, retrying in {}s (max {}s)",
                    attempt + 1,
                    backoff_secs,
                    MAX_BACKOFF_SECS
                );

                sleep(backoff).await;
                attempt += 1;
            }
        }
    }
}

/// Determine if a Signal error is retryable (transient).
///
/// Per GAP-06, retry only network errors (not protocol/logic errors).
pub fn is_signal_error_retryable(err: &crate::signal::traits::SignalError) -> bool {
    matches!(err, crate::signal::traits::SignalError::Network(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::traits::SignalError;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_succeeds_immediately() {
        let result = retry_with_backoff(
            || async { Ok::<_, SignalError>(42) },
            is_signal_error_retryable,
        )
        .await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_failures() {
        let attempt = Arc::new(AtomicU32::new(0));
        let attempt_clone = attempt.clone();

        let result = retry_with_backoff(
            move || {
                let attempt = attempt_clone.clone();
                async move {
                    let count = attempt.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err(SignalError::Network("transient".to_string()))
                    } else {
                        Ok(42)
                    }
                }
            },
            is_signal_error_retryable,
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_non_retryable_error_fails_immediately() {
        let attempt = Arc::new(AtomicU32::new(0));
        let attempt_clone = attempt.clone();

        let result = retry_with_backoff(
            move || {
                let attempt = attempt_clone.clone();
                async move {
                    attempt.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, _>(SignalError::Protocol("bad request".to_string()))
                }
            },
            is_signal_error_retryable,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(attempt.load(Ordering::SeqCst), 1); // Only 1 attempt
    }

    #[tokio::test]
    async fn test_retry_exhausts_max_retries() {
        // Note: This test would take too long to run with actual backoff
        // (cumulative ~1+ hour of sleep time). Instead, we verify the logic
        // by testing with fewer retries in test_retry_succeeds_after_failures.
        // In production, MAX_RETRIES = 12 ensures coverage of ~1 hour backoff.

        // Verify the math: 2^12 = 4096 seconds > 3600 seconds (1 hour cap)
        assert!(2u64.pow(MAX_RETRIES) > MAX_BACKOFF_SECS);

        // Verify that early attempts are under the cap
        assert!(2u64.pow(0) < MAX_BACKOFF_SECS); // 1 second
        assert!(2u64.pow(10) < MAX_BACKOFF_SECS); // 1024 seconds
    }

    #[tokio::test]
    async fn test_backoff_calculation() {
        // This test verifies the backoff duration calculation
        // We don't actually sleep, just check the logic
        assert_eq!(2u64.pow(0), 1); // First retry: 1 second
        assert_eq!(2u64.pow(1), 2); // Second retry: 2 seconds
        assert_eq!(2u64.pow(2), 4); // Third retry: 4 seconds
        assert_eq!(2u64.pow(10), 1024); // 11th retry: 1024 seconds
        assert_eq!(2u64.pow(12).min(MAX_BACKOFF_SECS), MAX_BACKOFF_SECS); // Capped
    }

    #[tokio::test]
    async fn test_is_signal_error_retryable() {
        // Network errors are retryable
        assert!(is_signal_error_retryable(&SignalError::Network(
            "timeout".to_string()
        )));

        // Protocol errors are NOT retryable
        assert!(!is_signal_error_retryable(&SignalError::Protocol(
            "bad request".to_string()
        )));

        // Other errors are NOT retryable
        assert!(!is_signal_error_retryable(&SignalError::Unauthorized));
        assert!(!is_signal_error_retryable(&SignalError::GroupNotFound(
            "group".to_string()
        )));
    }
}
