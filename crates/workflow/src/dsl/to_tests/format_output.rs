use super::safe_field;
use crate::dsl::from::FormatConfig;
use crate::dsl::to::{validate_to, DestinationConfig, OutputMode, ToDef};
use serde_json::json;
use std::collections::HashMap;

fn format_config(format_type: &str, options: serde_json::Value) -> FormatConfig {
    FormatConfig {
        format_type: format_type.to_string(),
        options,
    }
}

fn to_def(output: OutputMode, format: FormatConfig, mapping: HashMap<String, String>) -> ToDef {
    ToDef::Format {
        output,
        format,
        mapping,
    }
}

#[test]
fn format_type_empty_fails() {
    let def = to_def(
        OutputMode::Download,
        format_config("", json!({})),
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_err());
}

#[test]
fn csv_delimiter_multi_char_fails() {
    let def = to_def(
        OutputMode::Download,
        format_config("csv", json!({ "delimiter": ";;" })),
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_err());
}

#[test]
fn csv_delimiter_single_char_ok() {
    let def = to_def(
        OutputMode::Download,
        format_config("csv", json!({ "delimiter": ";" })),
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_ok());
}

#[test]
fn csv_escape_multi_char_fails() {
    let def = to_def(
        OutputMode::Download,
        format_config("csv", json!({ "escape": "\\\\" })),
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_err());
}

#[test]
fn csv_escape_empty_string_ok() {
    // Empty escape is explicitly allowed (means "no escape configured")
    let def = to_def(
        OutputMode::Download,
        format_config("csv", json!({ "escape": "" })),
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_ok());
}

#[test]
fn csv_quote_multi_char_fails() {
    let def = to_def(
        OutputMode::Download,
        format_config("csv", json!({ "quote": "\"\"" })),
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_err());
}

#[test]
fn csv_quote_single_char_ok() {
    let def = to_def(
        OutputMode::Download,
        format_config("csv", json!({ "quote": "\"" })),
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_ok());
}

#[test]
fn json_format_ignores_csv_only_options() {
    // format_type != "csv" means delimiter/escape/quote checks are skipped entirely
    let def = to_def(
        OutputMode::Download,
        format_config(
            "json",
            json!({ "delimiter": "too-long-to-be-valid-for-csv" }),
        ),
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_ok());
}

#[test]
fn output_download_and_api_need_no_destination() {
    for mode in [OutputMode::Download, OutputMode::Api] {
        let def = to_def(mode, format_config("json", json!({})), HashMap::new());
        assert!(validate_to(0, &def, &safe_field()).is_ok());
    }
}

#[test]
fn push_empty_destination_type_fails() {
    let def = to_def(
        OutputMode::Push {
            destination: DestinationConfig {
                destination_type: String::new(),
                config: json!({}),
                auth: None,
            },
            method: None,
        },
        format_config("json", json!({})),
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_err());
}

#[test]
fn push_non_uri_destination_skips_uri_specific_validation() {
    // "sftp" destinations bypass the uri-only checks (missing config.uri is fine here)
    let def = to_def(
        OutputMode::Push {
            destination: DestinationConfig {
                destination_type: "sftp".to_string(),
                config: json!({}),
                auth: None,
            },
            method: None,
        },
        format_config("json", json!({})),
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_ok());
}

#[test]
fn push_with_none_auth_ok() {
    let def = to_def(
        OutputMode::Push {
            destination: DestinationConfig {
                destination_type: "sftp".to_string(),
                config: json!({}),
                auth: Some(crate::data::adapters::auth::AuthConfig::None),
            },
            method: None,
        },
        format_config("json", json!({})),
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_ok());
}

#[test]
fn push_with_invalid_auth_propagates_error() {
    let def = to_def(
        OutputMode::Push {
            destination: DestinationConfig {
                destination_type: "sftp".to_string(),
                config: json!({}),
                auth: Some(crate::data::adapters::auth::AuthConfig::ApiKey {
                    key: String::new(),
                    header_name: "X-API-Key".to_string(),
                }),
            },
            method: None,
        },
        format_config("json", json!({})),
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_err());
}

#[test]
fn format_mapping_unsafe_key_fails() {
    let def = to_def(
        OutputMode::Download,
        format_config("json", json!({})),
        HashMap::from([("bad field!".to_string(), "source".to_string())]),
    );
    assert!(validate_to(0, &def, &safe_field()).is_err());
}

// --- uri push destinations -------------------------------------------------
//
// Regression cover for a dead branch: the destination-type comparison read
// `"uriformat!("` (a botched anyhow!-to-format!- find/replace), so a `uri` push
// destination never reached any of these checks. A workflow could ship an
// absent, empty or non-http URI and the DSL would accept it — the SSRF guard on
// the adapter was the only thing left standing between it and the network.

fn uri_push(config: serde_json::Value) -> ToDef {
    to_def(
        OutputMode::Push {
            destination: DestinationConfig {
                destination_type: "uri".to_string(),
                config,
                auth: None,
            },
            method: None,
        },
        format_config("json", json!({})),
        HashMap::new(),
    )
}

#[test]
fn uri_push_accepts_http_and_https() {
    for uri in ["http://example.com/hook", "https://example.com/hook"] {
        let def = uri_push(json!({ "uri": uri }));
        assert!(
            validate_to(0, &def, &safe_field()).is_ok(),
            "{uri} should be accepted"
        );
    }
}

#[test]
fn uri_push_requires_a_uri() {
    let def = uri_push(json!({}));
    assert!(
        validate_to(0, &def, &safe_field()).is_err(),
        "a uri destination with no uri must be rejected"
    );
}

#[test]
fn uri_push_rejects_an_empty_uri() {
    for value in ["", "   "] {
        let def = uri_push(json!({ "uri": value }));
        assert!(
            validate_to(0, &def, &safe_field()).is_err(),
            "a blank uri must be rejected"
        );
    }
}

/// The scheme check is what keeps a push destination on the network stack the
/// SSRF guard actually inspects.
#[test]
fn uri_push_rejects_non_http_schemes() {
    for uri in [
        "ftp://example.com/x",
        "file:///etc/passwd",
        "gopher://example.com",
        "//example.com/x",
        "example.com/x",
    ] {
        let def = uri_push(json!({ "uri": uri }));
        assert!(
            validate_to(0, &def, &safe_field()).is_err(),
            "{uri} must be rejected: only http:// and https:// are allowed"
        );
    }
}

/// A scheme that merely starts with the right letters must not slip through.
#[test]
fn uri_push_rejects_lookalike_schemes() {
    for uri in [
        "httpx://example.com",
        "https:/example.com",
        "http:example.com",
    ] {
        let def = uri_push(json!({ "uri": uri }));
        assert!(
            validate_to(0, &def, &safe_field()).is_err(),
            "{uri} must be rejected"
        );
    }
}

#[test]
fn uri_push_rejects_an_empty_destination_type() {
    let def = to_def(
        OutputMode::Push {
            destination: DestinationConfig {
                destination_type: "  ".to_string(),
                config: json!({ "uri": "https://example.com" }),
                auth: None,
            },
            method: None,
        },
        format_config("json", json!({})),
        HashMap::new(),
    );
    assert!(validate_to(0, &def, &safe_field()).is_err());
}
