use actix_web::HttpRequest;

/// Max failed login attempts per IP within [`WINDOW_SECS`].
pub const MAX_ATTEMPTS: u32 = 10;
/// Sliding window length, seconds.
pub const WINDOW_SECS: u64 = 900;

/// Derive the rate-limit cache key for a request's client IP.
#[must_use]
pub fn rate_limit_key(req: &HttpRequest) -> String {
    let ip = req
        .peer_addr()
        .map_or_else(|| "unknown".to_string(), |a| a.ip().to_string());
    format!("login_rl:{ip}")
}

/// Returns `true` when `attempts` has reached the limit.
#[must_use]
pub const fn is_rate_limited(attempts: u32) -> bool {
    attempts >= MAX_ATTEMPTS
}

#[cfg(test)]
mod tests {
    use super::{is_rate_limited, MAX_ATTEMPTS};

    #[test]
    fn under_limit_is_allowed() {
        assert!(!is_rate_limited(0));
        assert!(!is_rate_limited(MAX_ATTEMPTS - 1));
    }

    #[test]
    fn at_or_over_limit_is_blocked() {
        assert!(is_rate_limited(MAX_ATTEMPTS));
        assert!(is_rate_limited(MAX_ATTEMPTS + 5));
    }
}
