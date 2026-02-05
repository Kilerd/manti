use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Sliding window for tracking request timestamps
struct SlidingWindow {
    timestamps: Vec<Instant>,
}

impl SlidingWindow {
    fn new() -> Self {
        Self {
            timestamps: Vec::new(),
        }
    }

    /// Count requests within the window, removing expired ones
    fn count_in_window(&mut self, window_size: Duration) -> usize {
        let now = Instant::now();
        let cutoff = now - window_size;
        self.timestamps.retain(|&t| t > cutoff);
        self.timestamps.len()
    }

    /// Add a new request timestamp
    fn add_request(&mut self) {
        self.timestamps.push(Instant::now());
    }

    /// Get the oldest timestamp for retry-after calculation
    fn oldest_timestamp(&self) -> Option<Instant> {
        self.timestamps.first().copied()
    }
}

/// User-level sliding window rate limiter
pub struct RateLimiter {
    windows: RwLock<HashMap<Uuid, SlidingWindow>>,
    window_size: Duration,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            windows: RwLock::new(HashMap::new()),
            window_size: Duration::from_secs(60),
        }
    }

    /// Check if request is allowed under rate limit and record it if allowed.
    /// Returns Ok(()) if allowed, Err(retry_after_secs) if rate limited.
    pub fn check_and_record(&self, user_id: Uuid, limit_rpm: i32) -> Result<(), u64> {
        let mut windows = self.windows.write().unwrap();
        let window = windows.entry(user_id).or_insert_with(SlidingWindow::new);

        let current_count = window.count_in_window(self.window_size);

        if current_count >= limit_rpm as usize {
            // Calculate retry-after based on oldest request in window
            let retry_after = if let Some(oldest) = window.oldest_timestamp() {
                let elapsed = oldest.elapsed();
                if elapsed < self.window_size {
                    (self.window_size - elapsed).as_secs().max(1)
                } else {
                    1
                }
            } else {
                60
            };
            return Err(retry_after);
        }

        window.add_request();
        Ok(())
    }

    /// Check rate limit without recording (for pre-flight checks)
    pub fn check_only(&self, user_id: Uuid, limit_rpm: i32) -> Result<(), u64> {
        let mut windows = self.windows.write().unwrap();
        let window = windows.entry(user_id).or_insert_with(SlidingWindow::new);

        let current_count = window.count_in_window(self.window_size);

        if current_count >= limit_rpm as usize {
            let retry_after = if let Some(oldest) = window.oldest_timestamp() {
                let elapsed = oldest.elapsed();
                if elapsed < self.window_size {
                    (self.window_size - elapsed).as_secs().max(1)
                } else {
                    1
                }
            } else {
                60
            };
            return Err(retry_after);
        }

        Ok(())
    }

    /// Periodic cleanup of old entries to prevent memory growth
    pub fn cleanup(&self) {
        let mut windows = self.windows.write().unwrap();
        windows.retain(|_, window| window.count_in_window(self.window_size) > 0);
    }

    /// Get current request count for a user (for monitoring)
    pub fn get_current_count(&self, user_id: Uuid) -> usize {
        let mut windows = self.windows.write().unwrap();
        if let Some(window) = windows.get_mut(&user_id) {
            window.count_in_window(self.window_size)
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new();
        let user_id = Uuid::new_v4();

        // Should allow up to limit
        for _ in 0..10 {
            assert!(limiter.check_and_record(user_id, 10).is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new();
        let user_id = Uuid::new_v4();

        // Fill up the limit
        for _ in 0..5 {
            assert!(limiter.check_and_record(user_id, 5).is_ok());
        }

        // Should block the next request
        assert!(limiter.check_and_record(user_id, 5).is_err());
    }

    #[test]
    fn test_different_users_have_separate_limits() {
        let limiter = RateLimiter::new();
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();

        // Fill up user1's limit
        for _ in 0..5 {
            assert!(limiter.check_and_record(user1, 5).is_ok());
        }

        // user2 should still be allowed
        assert!(limiter.check_and_record(user2, 5).is_ok());
    }
}
