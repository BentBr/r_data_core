#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

//! Log-safe rendering of attacker-controlled values on the OIDC routes.

/// An OAuth error code as the provider sent it, or as much of it as is safe
/// to put in a log line: lowercase ASCII, digits and underscores, at most 32
/// characters. The parameter is attacker-chosen.
pub(super) fn safe_reason(raw: &str) -> String {
    let kept: String = raw
        .chars()
        .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
        .take(32)
        .collect();
    if kept.is_empty() {
        "<empty>".to_string()
    } else {
        kept
    }
}

#[cfg(test)]
mod tests {
    use super::safe_reason;

    #[test]
    fn a_provider_error_code_is_kept() {
        assert_eq!(safe_reason("access_denied"), "access_denied");
    }

    #[test]
    fn anything_that_is_not_an_error_code_is_not_logged_verbatim() {
        // The parameter is attacker-chosen. Newlines forge log lines; length
        // fills disks; anything outside the OAuth error-code alphabet is noise.
        assert_eq!(safe_reason("evil\nFAKE LOG LINE"), "evil");
        assert_eq!(safe_reason("x".repeat(500).as_str()).len(), 32);
        assert_eq!(safe_reason(""), "<empty>");
    }
}
