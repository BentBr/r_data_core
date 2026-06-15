use crate::error::{Error, Result};
use crate::field::definition::FieldDefinition;

impl FieldDefinition {
    /// Validate numeric constraints (min, max, `positive_only`)
    pub(super) fn validate_numeric_constraints(&self, n: f64) -> Result<()> {
        // Check min value
        if let Some(min_value) = &self.validation.min_value {
            if let Some(min) = min_value.as_f64() {
                if n < min {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be at least {}",
                        self.name, min
                    )));
                }
            }
        }

        // Check max value
        if let Some(max_value) = &self.validation.max_value {
            if let Some(max) = max_value.as_f64() {
                if n > max {
                    return Err(Error::Validation(format!(
                        "Field '{}' must be at most {}",
                        self.name, max
                    )));
                }
            }
        }

        // Check positive only
        if let Some(positive_only) = self.validation.positive_only {
            if positive_only && n < 0.0 {
                return Err(Error::Validation(format!(
                    "Field '{}' must be positive",
                    self.name
                )));
            }
        }

        Ok(())
    }

    /// Validate this field definition for common issues like invalid constraints
    ///
    /// # Errors
    /// Returns an error if validation fails
    #[allow(clippy::too_many_lines)] // Complex validation logic requires many lines
    pub fn validate(&self) -> Result<()> {
        // Check if the field has a valid name
        if self.name.is_empty() {
            return Err(Error::Validation("Field name cannot be empty".to_string()));
        }

        // Check for valid display name
        if self.display_name.is_empty() {
            return Err(Error::Validation(
                "Field display name cannot be empty".to_string(),
            ));
        }

        // Check for reserved SQL keywords
        let reserved_keywords = [
            "all",
            "analyse",
            "analyze",
            "and",
            "any",
            "array",
            "as",
            "asc",
            "asymmetric",
            "authorization",
            "binary",
            "both",
            "case",
            "cast",
            "check",
            "collate",
            "column",
            "constraint",
            "create",
            "cross",
            "current_date",
            "current_role",
            "current_time",
            "current_timestamp",
            "current_user",
            "default",
            "deferrable",
            "desc",
            "distinct",
            "do",
            "else",
            "end",
            "except",
            "false",
            "for",
            "foreign",
            "freeze",
            "from",
            "full",
            "grant",
            "group",
            "having",
            "in",
            "initially",
            "inner",
            "intersect",
            "into",
            "is",
            "isnull",
            "join",
            "leading",
            "left",
            "like",
            "limit",
            "localtime",
            "localtimestamp",
            "natural",
            "not",
            "notnull",
            "null",
            "offset",
            "on",
            "only",
            "or",
            "order",
            "outer",
            "overlaps",
            "placing",
            "primary",
            "references",
            "right",
            "select",
            "session_user",
            "similar",
            "some",
            "symmetric",
            "table",
            "then",
            "to",
            "trailing",
            "true",
            "union",
            "unique",
            "user",
            "using",
            "when",
            "where",
            "with",
        ];

        if reserved_keywords.contains(&self.name.to_lowercase().as_str()) {
            return Err(Error::Validation(format!(
                "Field name '{}' is a reserved SQL keyword and cannot be used",
                self.name
            )));
        }

        // Validate constraints based on field type
        for (constraint_type, constraint_value) in &self.constraints {
            self.handle_constraint(constraint_type, constraint_value)?;
        }

        Ok(())
    }
}
