use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Custom deserializer for converting string query parameters to i64
fn deserialize_optional_i64<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let value = Option::<String>::deserialize(deserializer)?;
    value.map_or_else(
        || Ok(None),
        |s| {
            s.parse::<i64>().map(Some).map_err(|_| {
                D::Error::custom(format!("invalid type: string \"{s}\", expected i64"))
            })
        },
    )
}

/// Flexible pagination query parameters that support both `page`/`per_page` and `limit`/`offset`
///
/// This struct provides a unified pagination interface that supports two common pagination patterns:
///
/// 1. **Page-based pagination**: Use `page` and `per_page` parameters
///    - `page`: Page number (1-based, default: 1)
///    - `per_page`: Number of items per page (default: 20, max: 100)
///
/// 2. **Offset-based pagination**: Use `limit` and `offset` parameters
///    - `limit`: Maximum number of items to return (default: 20, max: 100)
///    - `offset`: Number of items to skip (default: 0)
///
/// All parameters are optional and have sensible defaults. You can mix and match these parameters
/// as needed for your use case.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationQuery {
    /// Page number (1-based) - defaults to 1
    /// Use with `per_page` for page-based pagination
    #[serde(deserialize_with = "deserialize_optional_i64", default)]
    pub page: Option<i64>,

    /// Items per page - defaults to 20, max 100
    /// Use with `page` for page-based pagination
    #[serde(deserialize_with = "deserialize_optional_i64", default)]
    pub per_page: Option<i64>,

    /// Limit (alternative to `per_page`) - defaults to 20, max 100
    /// Use with `offset` for offset-based pagination
    #[serde(deserialize_with = "deserialize_optional_i64", default)]
    pub limit: Option<i64>,

    /// Offset (alternative to page) - defaults to 0
    /// Use with `limit` for offset-based pagination
    #[serde(deserialize_with = "deserialize_optional_i64", default)]
    pub offset: Option<i64>,
}

impl PaginationQuery {
    /// Validate pagination parameters
    ///
    /// # Returns
    /// - `Ok(())` if all parameters are valid
    /// - `Err(String)` with error message if invalid
    /// # Errors
    /// Returns an error if pagination parameters are invalid
    pub fn validate(&self, max_limit: i64, allow_unlimited: bool) -> Result<(), String> {
        // Validate page
        if let Some(page) = self.page {
            if page < 1 {
                return Err(format!("Invalid page number: must be >= 1, got {page}"));
            }
        }

        // Validate per_page
        if let Some(per_page) = self.per_page {
            if per_page == -1 {
                if !allow_unlimited {
                    return Err("per_page = -1 is not allowed for this endpoint".to_string());
                }
            } else if per_page < 1 || per_page > max_limit {
                return Err(format!(
                    "Invalid per_page: must be between 1 and {max_limit}, or -1 for unlimited, got {per_page}"
                ));
            }
        }

        // Validate limit
        if let Some(limit) = self.limit {
            if limit < 1 {
                return Err(format!("Invalid limit: must be >= 1, got {limit}"));
            }
            if limit > max_limit {
                return Err(format!(
                    "Invalid limit: must be between 1 and {max_limit}, got {limit}"
                ));
            }
        }

        // Validate offset
        if let Some(offset) = self.offset {
            if offset < 0 {
                return Err(format!("Invalid offset: must be >= 0, got {offset}"));
            }
        }

        Ok(())
    }

    /// Convert to limit/offset with defaults
    /// Returns (limit, offset) where limit = -1 means unlimited (skip LIMIT clause)
    #[must_use]
    pub fn to_limit_offset(&self, default_limit: i64, max_limit: i64) -> (i64, i64) {
        let (limit, offset) = if let (Some(page), Some(per_page)) = (self.page, self.per_page) {
            // Use page/per_page if both are provided
            let page = page.max(1);
            if per_page == -1 {
                // Unlimited
                (-1, 0)
            } else {
                let per_page = per_page.clamp(1, max_limit);
                let offset = (page - 1) * per_page;
                (per_page, offset)
            }
        } else if let (Some(limit_val), Some(offset)) = (self.limit, self.offset) {
            // Use limit/offset if both are provided
            let limit_val = limit_val.clamp(1, max_limit);
            let offset = offset.max(0);
            (limit_val, offset)
        } else if let Some(limit_val) = self.limit {
            // Use limit only
            let limit_val = limit_val.clamp(1, max_limit);
            let offset = self.offset.unwrap_or(0).max(0);
            (limit_val, offset)
        } else if let Some(per_page) = self.per_page {
            // Use per_page only
            if per_page == -1 {
                // Unlimited
                (-1, 0)
            } else {
                let per_page = per_page.clamp(1, max_limit);
                let page = self.page.unwrap_or(1).max(1);
                let offset = (page - 1) * per_page;
                (per_page, offset)
            }
        } else {
            // Use defaults
            let limit = default_limit.clamp(1, max_limit);
            let offset = self.offset.unwrap_or(0).max(0);
            (limit, offset)
        };

        (limit, offset)
    }

    /// Get the page number (for response metadata)
    #[must_use]
    pub fn get_page(&self, default: i64) -> i64 {
        self.page.map_or_else(
            || {
                if let (Some(limit), Some(offset)) = (self.limit, self.offset) {
                    // Calculate page from limit/offset
                    if limit > 0 {
                        (offset / limit) + 1
                    } else {
                        default
                    }
                } else if let Some(limit) = self.limit {
                    // If only limit is provided, use offset or default to 0
                    let offset = self.offset.unwrap_or(0);
                    if limit > 0 {
                        (offset / limit) + 1
                    } else {
                        default
                    }
                } else if let Some(_per_page) = self.per_page {
                    // If per_page is provided, calculate from page or use default
                    let page = self.page.unwrap_or(default);
                    page.max(1)
                } else {
                    default
                }
            },
            |page| page.max(1),
        )
    }

    /// Get the `per_page` value (for response metadata)
    /// Returns -1 if `per_page` was -1 (unlimited), otherwise returns clamped value
    #[must_use]
    pub fn get_per_page(&self, default: i64, max_limit: i64) -> i64 {
        self.per_page.map_or_else(
            || {
                self.limit.map_or_else(
                    || default.clamp(1, max_limit),
                    |limit| {
                        if limit == -1 {
                            -1
                        } else {
                            limit.clamp(1, max_limit)
                        }
                    },
                )
            },
            |per_page| {
                if per_page == -1 {
                    -1
                } else {
                    per_page.clamp(1, max_limit)
                }
            },
        )
    }
}

/// Standard sorting query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct SortingQuery {
    /// Field to sort by
    pub sort_by: Option<String>,
    /// Sort order (asc or desc)
    pub sort_order: Option<String>,
}

impl SortingQuery {
    /// Validate and get the sort order as uppercase
    ///
    /// # Returns
    /// - `Ok(String)` with "ASC" or "DESC" if valid
    /// - `Err(String)` with error message if invalid
    /// # Errors
    /// Returns an error if `sort_order` is not "asc" or "desc"
    pub fn validate_sort_order(&self) -> Result<String, String> {
        self.sort_order.as_ref().map_or_else(
            || Ok("ASC".to_string()),
            |order| {
                let order_upper = order.to_uppercase();
                match order_upper.as_str() {
                    "ASC" | "DESC" => Ok(order_upper),
                    _ => Err(format!(
                        "Invalid sort_order: '{order}'. Must be 'asc' or 'desc'"
                    )),
                }
            },
        )
    }

    /// Get the sort order as uppercase (defaults to ASC)
    /// This method does not validate - use `validate_sort_order` for validation
    #[must_use]
    pub fn get_sort_order(&self) -> String {
        self.validate_sort_order()
            .unwrap_or_else(|_| "ASC".to_string())
    }

    /// Sanitize a field name to prevent SQL injection
    /// Only allows alphanumeric characters and underscores
    ///
    /// # Returns
    /// - `Ok(String)` if the field name is valid
    /// - `Err(String)` if the field name contains invalid characters
    /// # Errors
    /// Returns an error if the field name is empty or contains invalid characters
    pub fn sanitize_field_name(field: &str) -> Result<String, String> {
        if field.is_empty() {
            return Err("Field name cannot be empty".to_string());
        }

        // Only allow alphanumeric characters and underscores
        if field.chars().all(|c| c.is_alphanumeric() || c == '_') {
            Ok(field.to_string())
        } else {
            Err(format!(
                "Invalid field name: '{field}'. Field names can only contain alphanumeric characters and underscores"
            ))
        }
    }

    /// Get the sort SQL clause if `sort_by` is provided
    /// This method does not validate the field - validation should be done separately
    #[must_use]
    pub fn get_sort_clause(&self) -> Option<String> {
        self.sort_by.as_ref().map(|field| {
            let order = self.get_sort_order();
            format!("{field} {order}")
        })
    }
}

/// Field selection query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct FieldsQuery {
    /// Comma-separated list of fields to include
    pub fields: Option<String>,
}

impl FieldsQuery {
    /// Parse fields into a vector of field names
    #[must_use]
    pub fn get_fields(&self) -> Option<Vec<String>> {
        self.fields.as_ref().map(|fields| {
            fields
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
    }
}

/// Filter query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct FilterQuery {
    /// JSON filter object or compact filter syntax
    pub filter: Option<String>,
    /// Search query
    pub q: Option<String>,
}

impl FilterQuery {
    /// Parse the filter string into a structured filter
    #[must_use]
    pub fn parse_filter(&self) -> Option<Value> {
        self.filter.as_ref().map(|filter_str| {
            // Try to parse as JSON first
            serde_json::from_str(filter_str).unwrap_or_else(|_| {
                // If that fails, treat as compact filter syntax
                // Example: "status:active,type:user"
                let mut map = HashMap::new();
                for part in filter_str.split(',') {
                    if let Some((key, value)) = part.split_once(':') {
                        map.insert(
                            key.trim().to_string(),
                            Value::String(value.trim().to_string()),
                        );
                    }
                }
                Value::Object(serde_json::Map::from_iter(map))
            })
        })
    }
}

/// Include related entities query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct IncludeQuery {
    /// Comma-separated list of related entities to include
    pub include: Option<String>,
    /// Whether to include the count of child entities in the response
    pub include_children_count: Option<bool>,
}

impl IncludeQuery {
    /// Parse include into a vector of relation names
    #[must_use]
    pub fn get_includes(&self) -> Option<Vec<String>> {
        self.include.as_ref().map(|include| {
            include
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
    }

    /// Check if children count should be included
    #[must_use]
    pub fn should_include_children_count(&self) -> bool {
        self.include_children_count.unwrap_or(false)
    }
}

/// Comprehensive standardized query parameters for API endpoints
///
/// This struct provides a unified interface for handling various query parameters
/// including pagination, sorting, filtering, and field selection. It supports
/// flexible pagination through the `PaginationQuery` struct.
#[derive(Debug, Deserialize, ToSchema)]
pub struct StandardQuery {
    #[serde(flatten)]
    pub pagination: PaginationQuery,

    #[serde(flatten)]
    pub sorting: SortingQuery,

    #[serde(flatten)]
    pub fields: FieldsQuery,

    #[serde(flatten)]
    pub filter: FilterQuery,

    #[serde(flatten)]
    pub include: IncludeQuery,
}

impl StandardQuery {
    /// Convert to limit/offset with defaults
    #[must_use]
    pub fn to_limit_offset(&self) -> (i64, i64) {
        self.pagination.to_limit_offset(1, 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_pagination_query_methods() {
        let query = PaginationQuery {
            page: Some(2),
            per_page: Some(50),
            limit: None,
            offset: None,
        };

        // Test get_page with default
        assert_eq!(query.get_page(1), 2);
        assert_eq!(query.get_page(10), 2);

        // Test get_per_page with default and max limit
        assert_eq!(query.get_per_page(20, 100), 50);
        assert_eq!(query.get_per_page(10, 30), 30); // Should be clamped to max

        // Test to_limit_offset
        let (limit, offset) = query.to_limit_offset(1, 100);
        assert_eq!(limit, 50);
        assert_eq!(offset, 50); // (page - 1) * per_page = (2 - 1) * 50 = 50
    }

    #[test]
    fn test_pagination_query_with_none_values() {
        let query = PaginationQuery {
            page: None,
            per_page: None,
            limit: None,
            offset: None,
        };

        // Test get_page with default
        assert_eq!(query.get_page(1), 1);
        assert_eq!(query.get_page(10), 10);

        // Test get_per_page with default and max limit
        assert_eq!(query.get_per_page(20, 100), 20);
        assert_eq!(query.get_per_page(10, 30), 10);

        // Test to_limit_offset
        let (limit, offset) = query.to_limit_offset(20, 100);
        assert_eq!(limit, 20);
        assert_eq!(offset, 0); // (page - 1) * per_page = (1 - 1) * 20 = 0
    }

    #[test]
    fn test_pagination_query_edge_cases() {
        // Test page 0 (should be clamped to 1)
        let query = PaginationQuery {
            page: Some(0),
            per_page: Some(10),
            limit: None,
            offset: None,
        };
        assert_eq!(query.get_page(1), 1); // Should be clamped to minimum 1

        // Test per_page 0 (should be clamped to 1)
        let query = PaginationQuery {
            page: Some(1),
            per_page: Some(0),
            limit: None,
            offset: None,
        };
        assert_eq!(query.get_per_page(20, 100), 1); // Should be clamped to minimum 1

        // Test very large numbers
        let query = PaginationQuery {
            page: Some(999_999),
            per_page: Some(999_999),
            limit: None,
            offset: None,
        };
        assert_eq!(query.get_page(1), 999_999);
        assert_eq!(query.get_per_page(20, 100), 100); // Should be clamped to max 100
    }

    #[test]
    fn test_pagination_query_manual_construction() {
        // Test manually constructed PaginationQuery with string-converted values
        let query = PaginationQuery {
            page: Some(1),
            per_page: Some(1000),
            limit: None,
            offset: None,
        };

        assert_eq!(query.page, Some(1));
        assert_eq!(query.per_page, Some(1000));

        // Test the methods work correctly
        assert_eq!(query.get_page(1), 1);
        assert_eq!(query.get_per_page(20, 100), 100); // Should be clamped to max 100
    }

    #[test]
    fn test_deserializer_integration() {
        // Test that the deserializer works with the actual struct
        // This simulates what happens when query parameters are deserialized

        // Test with string values (simulating query parameters)
        let json = serde_json::json!({
            "page": "1",
            "per_page": "1000",
            "limit": null,
            "offset": null
        });

        // This should work because our deserializer handles string-to-i64 conversion
        let result: PaginationQuery = serde_json::from_value(json).unwrap();
        assert_eq!(result.page, Some(1));
        assert_eq!(result.per_page, Some(1000));
    }

    #[test]
    fn test_limit_offset_parameters() {
        // Test with limit/offset parameters
        let json = serde_json::json!({
            "limit": "50",
            "offset": "100",
            "page": null,
            "per_page": null
        });

        let result: PaginationQuery = serde_json::from_value(json).unwrap();
        assert_eq!(result.limit, Some(50));
        assert_eq!(result.offset, Some(100));
        assert_eq!(result.page, None);
        assert_eq!(result.per_page, None);
    }

    #[test]
    fn test_mixed_parameters() {
        // Test with mixed parameters (should prioritize page/per_page)
        let json = serde_json::json!({
            "page": "2",
            "per_page": "25",
            "limit": "50",
            "offset": "100"
        });

        let result: PaginationQuery = serde_json::from_value(json).unwrap();
        assert_eq!(result.page, Some(2));
        assert_eq!(result.per_page, Some(25));
        assert_eq!(result.limit, Some(50));
        assert_eq!(result.offset, Some(100));
    }
}
