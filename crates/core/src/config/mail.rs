#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::Result;

/// Configuration for a single SMTP connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    /// SMTP server hostname
    pub host: String,
    /// SMTP server port
    pub port: u16,
    /// Optional SMTP username for authentication
    pub username: Option<String>,
    /// Optional SMTP password for authentication
    pub password: Option<String>,
    /// Whether to use TLS
    pub tls: bool,
    /// Sender e-mail address (the `From:` header)
    pub from_address: String,
    /// Optional display name used in the `From:` header
    pub from_name: Option<String>,
}

/// Mail configuration holding optional SMTP configs for system and workflow e-mails
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MailConfig {
    /// SMTP configuration used for system e-mails (e.g. password-reset)
    pub system: Option<SmtpConfig>,
    /// SMTP configuration used for workflow-triggered e-mails
    pub workflow: Option<SmtpConfig>,
}

/// Parse an SMTP DSN string into an [`SmtpConfig`].
///
/// DSN format:
/// `smtp://[user:pass@]host:port?tls=true|false&from=addr@example.com[&from_name=Display%20Name]`
///
/// - `user:pass@` is optional
/// - `tls` defaults to `false` when omitted
/// - `from` query parameter is **required**
/// - `from_name` is optional and will be URL-decoded
///
/// # Errors
///
/// Returns [`crate::error::Error::Config`] when:
/// - The DSN cannot be parsed as a URL
/// - The host is empty or missing
/// - The `from` query parameter is missing or empty
/// - The port cannot be determined
pub fn parse_smtp_dsn(dsn: &str) -> Result<SmtpConfig> {
    let url = Url::parse(dsn)
        .map_err(|e| crate::error::Error::Config(format!("Invalid SMTP DSN: {e}")))?;

    let host = url.host_str().unwrap_or("").to_string();
    if host.is_empty() {
        return Err(crate::error::Error::Config(
            "SMTP DSN must contain a non-empty host".to_string(),
        ));
    }

    let port = url
        .port()
        .ok_or_else(|| crate::error::Error::Config("SMTP DSN must contain a port".to_string()))?;

    let username = {
        let u = url.username();
        if u.is_empty() {
            None
        } else {
            Some(u.to_string())
        }
    };

    let password = url.password().map(ToString::to_string);

    let mut tls = false;
    let mut from_address: Option<String> = None;
    let mut from_name: Option<String> = None;

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "tls" => {
                tls = value.eq_ignore_ascii_case("true");
            }
            "from" => {
                from_address = Some(
                    urlencoding::decode(&value)
                        .map_or_else(|_| value.to_string(), std::borrow::Cow::into_owned),
                );
            }
            "from_name" => {
                from_name = Some(
                    urlencoding::decode(&value)
                        .map_or_else(|_| value.to_string(), std::borrow::Cow::into_owned),
                );
            }
            _ => {}
        }
    }

    let from_address = from_address.filter(|s| !s.is_empty()).ok_or_else(|| {
        crate::error::Error::Config(
            "SMTP DSN must contain a non-empty `from` query parameter".to_string(),
        )
    })?;

    Ok(SmtpConfig {
        host,
        port,
        username,
        password,
        tls,
        from_address,
        from_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dsn_full_auth() {
        let dsn =
            "smtp://user:pass@mail.example.com:587?tls=true&from=a@b.com&from_name=Test%20System";
        let config = parse_smtp_dsn(dsn).unwrap();
        assert_eq!(config.host, "mail.example.com");
        assert_eq!(config.port, 587);
        assert_eq!(config.username.as_deref(), Some("user"));
        assert_eq!(config.password.as_deref(), Some("pass"));
        assert!(config.tls);
        assert_eq!(config.from_address, "a@b.com");
        assert_eq!(config.from_name.as_deref(), Some("Test System"));
    }

    #[test]
    fn parse_dsn_no_auth() {
        let dsn = "smtp://mailpit:1025?tls=false&from=noreply@local";
        let config = parse_smtp_dsn(dsn).unwrap();
        assert_eq!(config.host, "mailpit");
        assert_eq!(config.port, 1025);
        assert!(config.username.is_none());
        assert!(config.password.is_none());
        assert!(!config.tls);
        assert_eq!(config.from_address, "noreply@local");
        assert!(config.from_name.is_none());
    }

    #[test]
    fn parse_dsn_missing_from_fails() {
        let dsn = "smtp://host:25?tls=false";
        assert!(parse_smtp_dsn(dsn).is_err());
    }

    #[test]
    fn parse_dsn_missing_host_fails() {
        let dsn = "smtp://:25?from=a@b.com";
        assert!(parse_smtp_dsn(dsn).is_err());
    }

    #[test]
    fn parse_dsn_default_tls_false() {
        let dsn = "smtp://host:25?from=a@b.com";
        let config = parse_smtp_dsn(dsn).unwrap();
        assert!(!config.tls);
    }

    #[test]
    fn parse_dsn_with_special_chars_in_password() {
        let dsn = "smtp://user:p%40ss%23word@host:587?tls=true&from=a@b.com";
        let config = parse_smtp_dsn(dsn).unwrap();
        assert_eq!(config.username.as_deref(), Some("user"));
        // URL crate decodes the password automatically
        assert!(config.password.is_some());
    }

    #[test]
    fn parse_dsn_non_smtp_scheme_parses_without_scheme_validation() {
        // Our parser does not validate the URL scheme; it only requires host, port, and `from`.
        // An http:// DSN with all required fields succeeds (scheme checking is not implemented).
        let dsn = "http://host:25?from=a@b.com";
        let config = parse_smtp_dsn(dsn).unwrap();
        assert_eq!(config.host, "host");
        assert_eq!(config.port, 25);
        assert_eq!(config.from_address, "a@b.com");
    }

    #[test]
    fn parse_dsn_from_name_with_spaces() {
        let dsn = "smtp://host:25?from=a@b.com&from_name=My%20Cool%20System";
        let config = parse_smtp_dsn(dsn).unwrap();
        assert_eq!(config.from_name.as_deref(), Some("My Cool System"));
    }
}
