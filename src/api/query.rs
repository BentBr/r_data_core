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
    match value {
        Some(s) => s
            .parse::<i64>()
            .map(Some)
            .map_err(|_| D::Error::custom(format!("invalid type: string \"{}\", expected i64", s))),
        None => Ok(None),
    }
}

/// Flexible pagination query parameters that support both page/per_page and limit/offset
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

    /// Limit (alternative to per_page) - defaults to 20, max 100
    /// Use with `offset` for offset-based pagination
    #[serde(deserialize_with = "deserialize_optional_i64", default)]
    pub limit: Option<i64>,

    /// Offset (alternative to page) - defaults to 0
    /// Use with `limit` for offset-based pagination
    #[serde(deserialize_with = "deserialize_optional_i64", default)]
    pub offset: Option<i64>,
}

impl PaginationQuery {
    /// Convert to limit/offset with defaults
    pub fn to_limit_offset(&self, default_limit: i64, max_limit: i64) -> (i64, i64) {
        let (limit, offset) = if let (Some(page), Some(per_page)) = (self.page, self.per_page) {
            // Use page/per_page if both are provided
            let page = page.max(1);
            let per_page = per_page.clamp(1, max_limit);
            let offset = (page - 1) * per_page;
            (per_page, offset)
        } else if let (Some(limit), Some(offset)) = (self.limit, self.offset) {
            // Use limit/offset if both are provided
            let limit = limit.clamp(1, max_limit);
            let offset = offset.max(0);
            (limit, offset)
        } else if let Some(limit) = self.limit {
            // Use limit only
            let limit = limit.clamp(1, max_limit);
            let offset = self.offset.unwrap_or(0).max(0);
            (limit, offset)
        } else if let Some(per_page) = self.per_page {
            // Use per_page only
            let per_page = per_page.clamp(1, max_limit);
            let page = self.page.unwrap_or(1).max(1);
            let offset = (page - 1) * per_page;
            (per_page, offset)
        } else {
            // Use defaults
            let limit = default_limit.clamp(1, max_limit);
            let offset = self.offset.unwrap_or(0).max(0);
            (limit, offset)
        };

        (limit, offset)
    }

    /// Get the page number (for response metadata)
    pub fn get_page(&self, default: i64) -> i64 {
        if let Some(page) = self.page {
            page.max(1)
        } else if let (Some(limit), Some(offset)) = (self.limit, self.offset) {
            // Calculate page from limit/offset
            if limit > 0 {
                (offset / limit) + 1
            } else {
                default
            }
        } else {
            default
        }
    }

    /// Get the per_page value (for response metadata)
    pub fn get_per_page(&self, default: i64, max_limit: i64) -> i64 {
        if let Some(per_page) = self.per_page {
            per_page.clamp(1, max_limit)
        } else if let Some(limit) = self.limit {
            limit.clamp(1, max_limit)
        } else {
            default.clamp(1, max_limit)
        }
    }
}

/// Standard sorting query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct SortingQuery {
    /// Field to sort by
    pub sort_by: Option<String>,
    /// Sort direction (asc or desc)
    pub sort_direction: Option<String>,
}

impl SortingQuery {
    /// Get the sort direction as uppercase
    pub fn get_sort_direction(&self) -> String {
        match &self.sort_direction {
            Some(dir) => {
                let dir = dir.to_uppercase();
                if dir == "ASC" || dir == "DESC" {
                    dir
                } else {
                    "ASC".to_string()
                }
            }
            None => "ASC".to_string(),
        }
    }

    /// Get the sort SQL clause if sort_by is provided
    pub fn get_sort_clause(&self) -> Option<String> {
        self.sort_by
            .as_ref()
            .map(|field| format!("{} {}", field, self.get_sort_direction()))
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
    pub fn parse_filter(&self) -> Option<Value> {
        match &self.filter {
            Some(filter_str) => {
                // Try to parse as JSON first
                serde_json::from_str(filter_str).ok().or_else(|| {
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
                    Some(Value::Object(serde_json::Map::from_iter(
                        map.into_iter().map(|(k, v)| (k, v)),
                    )))
                })
            }
            None => None,
        }
    }
}

/// Include related entities query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct IncludeQuery {
    /// Comma-separated list of related entities to include
    pub include: Option<String>,
}

impl IncludeQuery {
    /// Parse include into a vector of relation names
    pub fn get_includes(&self) -> Option<Vec<String>> {
        self.include.as_ref().map(|include| {
            include
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
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
            page: Some(999999),
            per_page: Some(999999),
            limit: None,
            offset: None,
        };
        assert_eq!(query.get_page(1), 999999);
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
