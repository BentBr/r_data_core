use serde::{Deserialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Standard pagination query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationQuery {
    /// Page number (1-based)
    pub page: Option<i64>,
    /// Number of items per page
    pub per_page: Option<i64>,
}

impl PaginationQuery {
    /// Get the page number with a default value
    pub fn get_page(&self, default: i64) -> i64 {
        self.page.unwrap_or(default).max(1)
    }

    /// Get the per_page value with a default value and max limit
    pub fn get_per_page(&self, default: i64, max_limit: i64) -> i64 {
        self.per_page.unwrap_or(default).clamp(1, max_limit)
    }

    /// Convert page/per_page to limit/offset
    pub fn to_limit_offset(
        &self,
        default_page: i64,
        default_per_page: i64,
        max_limit: i64,
    ) -> (i64, i64) {
        let page = self.get_page(default_page);
        let per_page = self.get_per_page(default_per_page, max_limit);
        let offset = (page - 1) * per_page;
        (per_page, offset)
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

/// Comprehensive standardized query parameters
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
        self.pagination.to_limit_offset(1, 20, 100)
    }
}
