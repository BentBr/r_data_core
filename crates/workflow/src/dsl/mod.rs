#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

mod csv;
mod execution;
mod from;
mod processor;
mod program;
mod to;
mod transform;
mod validation;

// pub use csv::CsvOptions; // TODO: Re-enable when used
pub use from::{EntityFilter, FormatConfig, FromDef, SourceConfig};
// pub use processor::DslProcessor; // TODO: Re-enable when used
pub use program::DslProgram;
pub use to::{EntityWriteMode, OutputMode, ToDef};
pub use transform::{
    ArithmeticOp, ArithmeticTransform, ConcatTransform, Operand, StringOperand, Transform,
};
pub use validation::validate_mapping;

/// Strict, explicit DSL step tying together from → transform → to
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct DslStep {
    /// Source definition
    pub from: FromDef,
    /// Target definition
    pub to: ToDef,
    /// Transform to apply
    pub transform: Transform,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_ok_and_apply_arithmetic_field_const() {
        let config = json!({
            "steps": [{
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example/csv" },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": {}
                    },
                    "mapping": { "price": "price" }
                },
                "transform": {
                    "type": "arithmetic",
                    "target": "price",
                    "left": { "kind": "field", "field": "price" },
                    "op": "add",
                    "right": { "kind": "const", "value": 5.0 }
                },
                // Mapping structure: { destination_field: normalized_field }
                // So "entity.total" (destination) maps from normalized "price"
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
                    "mapping": { "entity.total": "price" }
                }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        prog.validate().unwrap();

        let input = json!({ "price": 10.0 });
        let out = prog.apply(&input).unwrap();
        assert_eq!(out["entity"]["total"], json!(15.0));
    }

    #[test]
    fn test_mapping_destination_to_normalized() {
        // Test that mapping structure { destination_field: normalized_field } works correctly
        // This tests the fix where we swap (src, dst) to (dst, src) in the iteration
        let config = json!({
            "steps": [{
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.csv" },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": {}
                    },
                    "mapping": {
                        "email": "email",
                        "active": "active",
                        "firstName": "firstName",
                        "lastName": "lastName"
                    }
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": "Customer",
                    "path": "/test",
                    "mode": "create",
                    "mapping": {
                        "email": "email",
                        "published": "active",
                        "firstName": "firstName",
                        "lastName": "lastName",
                        "entity_key": "email"
                    }
                }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        prog.validate().unwrap();

        // Input has: email, active, firstName, lastName
        let input = json!({
            "email": "test@example.com",
            "active": true,
            "firstName": "John",
            "lastName": "Doe"
        });

        // Execute should produce: email, published (from active), firstName, lastName, entity_key (from email)
        let outputs = prog.execute(&input).unwrap();
        assert_eq!(outputs.len(), 1);

        let (to_def, produced) = &outputs[0];
        match to_def {
            ToDef::Entity { .. } => {
                // Verify that "active" was mapped to "published"
                assert_eq!(produced["published"], json!(true));
                // Verify that "email" was mapped to both "email" and "entity_key"
                assert_eq!(produced["email"], json!("test@example.com"));
                assert_eq!(produced["entity_key"], json!("test@example.com"));
                // Verify that "active" is NOT in the output (should be "published" instead)
                assert!(!produced.as_object().unwrap().contains_key("active"));
                // Verify other fields
                assert_eq!(produced["firstName"], json!("John"));
                assert_eq!(produced["lastName"], json!("Doe"));
            }
            _ => panic!("Expected Entity ToDef"),
        }
    }

    #[test]
    fn test_validate_from_api_without_endpoint() {
        // from.api without endpoint field should be valid (accepts POST)
        let config = json!({
            "steps": [{
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "api",
                        "config": {},
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
                    "mapping": {}
                }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        prog.validate().unwrap();
    }

    #[test]
    fn test_validate_from_api_with_endpoint_fails() {
        // from.api with endpoint field should fail validation
        let config = json!({
            "steps": [{
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "api",
                        "config": {
                            "endpoint": "/api/v1/workflows/test"
                        },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": { "has_header": true }
                    },
                    "mapping": {}
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "format",
                    "output": { "mode": "api" },
                    "format": {
                        "format_type": "json",
                        "options": {}
                    },
                    "mapping": {}
                }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        let result = prog.validate();
        assert!(
            result.is_err(),
            "Expected validation to fail for from.api with endpoint field"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("endpoint is not allowed"),
            "Error message should mention endpoint is not allowed"
        );
    }

    #[test]
    fn test_mapping_same_field_multiple_times() {
        // Test that the same normalized field can be mapped to multiple destination fields
        let config = json!({
            "steps": [{
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.csv" },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": {}
                    },
                    "mapping": {
                        "email": "email"
                    }
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": "Customer",
                    "path": "/test",
                    "mode": "create",
                    "mapping": {
                        "email": "email",
                        "entity_key": "email"
                    }
                }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        prog.validate().unwrap();

        let input = json!({
            "email": "test@example.com"
        });

        let outputs = prog.execute(&input).unwrap();
        assert_eq!(outputs.len(), 1);

        let (_, produced) = &outputs[0];
        // Both email and entity_key should have the same value
        assert_eq!(produced["email"], json!("test@example.com"));
        assert_eq!(produced["entity_key"], json!("test@example.com"));
    }

    #[test]
    fn test_mapping_apply_consistency() {
        // Test that apply() method has the same behavior as execute() for mapping
        let config = json!({
            "steps": [{
                "from": {
                    "type": "format",
                    "source": {
                        "source_type": "uri",
                        "config": { "uri": "http://example.com/data.csv" },
                        "auth": null
                    },
                    "format": {
                        "format_type": "csv",
                        "options": {}
                    },
                    "mapping": {
                        "active": "active"
                    }
                },
                "transform": { "type": "none" },
                "to": {
                    "type": "entity",
                    "entity_definition": "Customer",
                    "path": "/test",
                    "mode": "create",
                    "mapping": {
                        "published": "active"
                    }
                }
            }]
        });
        let prog = DslProgram::from_config(&config).unwrap();
        prog.validate().unwrap();

        let input = json!({
            "active": true
        });

        // Test apply()
        let out = prog.apply(&input).unwrap();
        assert_eq!(out["published"], json!(true));
        assert!(!out.as_object().unwrap().contains_key("active"));

        // Test execute() - should produce same result
        let outputs = prog.execute(&input).unwrap();
        assert_eq!(outputs.len(), 1);
        let (_, produced) = &outputs[0];
        assert_eq!(produced["published"], json!(true));
        assert!(!produced.as_object().unwrap().contains_key("active"));
    }
}
