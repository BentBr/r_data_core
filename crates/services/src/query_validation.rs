#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cached field validator for database tables
pub struct FieldValidator {
    pool: Arc<PgPool>,
    cache: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

impl FieldValidator {
    /// Create a new field validator
    #[must_use]
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            pool,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get valid fields for a table, using cache if available
    ///
    /// # Errors
    /// Returns an error if database query fails
    pub async fn get_valid_fields(&self, table_name: &str) -> Result<HashSet<String>, String> {
        // Check cache first
        {
            let cache: tokio::sync::RwLockReadGuard<'_, HashMap<String, HashSet<String>>> =
                self.cache.read().await;
            if let Some(fields) = cache.get(table_name) {
                return Ok(fields.clone());
            }
        }

        // Query database schema
        let fields = self.query_table_fields(table_name).await?;

        // Update cache
        {
            let mut cache: tokio::sync::RwLockWriteGuard<'_, HashMap<String, HashSet<String>>> =
                self.cache.write().await;
            cache.insert(table_name.to_string(), fields.clone());
        }

        Ok(fields)
    }

    /// Query database `information_schema` to get column names for a table
    ///
    /// # Errors
    /// Returns an error if the database query fails
    async fn query_table_fields(&self, table_name: &str) -> QueryValidationResult<HashSet<String>> {
        let rows = sqlx::query!(
            r#"
            SELECT column_name
            FROM information_schema.columns
            WHERE table_schema = current_schema() AND table_name = $1
            "#,
            table_name
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| format!("Failed to query table schema: {e}"))?;

        let fields: HashSet<String> = rows.into_iter().filter_map(|row| row.column_name).collect();

        Ok(fields)
    }

    /// Validate that a field exists in the table
    ///
    /// # Errors
    /// Returns an error if field doesn't exist or database query fails
    pub async fn validate_field(&self, table_name: &str, field_name: &str) -> Result<(), String> {
        // First sanitize the field name
        let sanitized = Self::sanitize_field_name(field_name)?;

        // Get valid fields
        let valid_fields = self.get_valid_fields(table_name).await?;

        if valid_fields.contains(&sanitized) {
            Ok(())
        } else {
            let valid_fields_list: Vec<String> = {
                let mut fields: Vec<String> = valid_fields.into_iter().collect();
                fields.sort();
                fields
            };
            Err(format!(
                "Invalid sort field: '{field_name}'. Valid fields are: {}",
                valid_fields_list.join(", ")
            ))
        }
    }

    /// Sanitize a field name to prevent SQL injection
    /// Only allows alphanumeric characters and underscores
    ///
    /// # Returns
    /// - `Ok(String)` if the field name is valid
    /// - `Err(String)` if the field name contains invalid characters
    ///
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

    /// Clear the cache for a specific table (useful for testing or schema changes)
    pub async fn clear_cache(&self, table_name: &str) {
        let mut cache: tokio::sync::RwLockWriteGuard<'_, HashMap<String, HashSet<String>>> =
            self.cache.write().await;
        cache.remove(table_name);
    }

    /// Clear all cached fields
    pub async fn clear_all_cache(&self) {
        let mut cache: tokio::sync::RwLockWriteGuard<'_, HashMap<String, HashSet<String>>> =
            self.cache.write().await;
        cache.clear();
    }
}

/// Result type for query validation
pub type QueryValidationResult<T> = Result<T, String>;

/// Query parameters for list operations (extracted from `StandardQuery`)
#[derive(Debug, Clone)]
pub struct ListQueryParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

/// Validated query parameters for list operations
#[derive(Debug, Clone)]
pub struct ValidatedListQuery {
    pub limit: i64,
    pub offset: i64,
    pub page: i64,
    pub per_page: i64,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

/// Validate and process query parameters for list operations
///
/// # Arguments
/// * `params` - The query parameters
/// * `table_name` - The database table name to validate sort fields against
/// * `field_validator` - The `FieldValidator` instance
/// * `default_limit` - Default limit if not specified
/// * `max_limit` - Maximum allowed limit
/// * `allow_unlimited` - Whether to allow `per_page` = -1 for unlimited results
///
/// # Errors
/// Returns an error if validation fails
pub async fn validate_list_query(
    params: &ListQueryParams,
    table_name: &str,
    field_validator: &FieldValidator,
    default_limit: i64,
    max_limit: i64,
    allow_unlimited: bool,
    allowed_virtual_fields: &[&str],
) -> QueryValidationResult<ValidatedListQuery> {
    // Create a temporary PaginationQuery for validation
    let pagination = PaginationQuery {
        page: params.page,
        per_page: params.per_page,
        limit: params.limit,
        offset: params.offset,
    };

    // Validate pagination
    pagination
        .validate(max_limit, allow_unlimited)
        .map_err(|e| format!("Pagination validation failed: {e}"))?;

    // Validate sort_order if provided
    if let Some(ref sort_order) = params.sort_order {
        let order_upper = sort_order.to_uppercase();
        if order_upper != "ASC" && order_upper != "DESC" {
            return Err(format!(
                "Invalid sort_order: '{sort_order}'. Must be 'asc' or 'desc'"
            ));
        }
    }

    // Validate sort_by field if provided
    if let Some(ref sort_by) = params.sort_by {
        // Allow whitelisted virtual fields (e.g., derived columns)
        if allowed_virtual_fields
            .iter()
            .any(|field| field == &sort_by.as_str())
        {
            // Still sanitize to avoid injection
            FieldValidator::sanitize_field_name(sort_by)
                .map_err(|e| format!("Sort field validation failed: {e}"))?;
        } else {
            field_validator
                .validate_field(table_name, sort_by)
                .await
                .map_err(|e| format!("Sort field validation failed: {e}"))?;
        }
    }

    // Convert to limit/offset
    let (limit, offset) = pagination.to_limit_offset(default_limit, max_limit);
    let page = pagination.get_page(1);
    let per_page = pagination.get_per_page(default_limit, max_limit);

    Ok(ValidatedListQuery {
        limit,
        offset,
        page,
        per_page,
        sort_by: params.sort_by.clone(),
        sort_order: params.sort_order.clone(),
    })
}

/// Pagination query parameters (duplicated from api crate to avoid dependency)
#[derive(Debug, Clone)]
struct PaginationQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl PaginationQuery {
    fn validate(&self, max_limit: i64, allow_unlimited: bool) -> Result<(), String> {
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

    fn to_limit_offset(&self, default_limit: i64, max_limit: i64) -> (i64, i64) {
        let (limit, offset) = if let (Some(page), Some(per_page)) = (self.page, self.per_page) {
            let page = page.max(1);
            if per_page == -1 {
                (i64::MAX, 0)
            } else {
                let per_page = per_page.clamp(1, max_limit);
                let offset = (page - 1) * per_page;
                (per_page, offset)
            }
        } else if let (Some(limit_val), Some(offset)) = (self.limit, self.offset) {
            let limit_val = limit_val.clamp(1, max_limit);
            let offset = offset.max(0);
            (limit_val, offset)
        } else if let Some(limit_val) = self.limit {
            let limit_val = limit_val.clamp(1, max_limit);
            let offset = self.offset.unwrap_or(0).max(0);
            (limit_val, offset)
        } else if let Some(per_page) = self.per_page {
            if per_page == -1 {
                (i64::MAX, 0)
            } else {
                let per_page = per_page.clamp(1, max_limit);
                let page = self.page.unwrap_or(1).max(1);
                let offset = (page - 1) * per_page;
                (per_page, offset)
            }
        } else {
            let limit = default_limit.clamp(1, max_limit);
            let offset = self.offset.unwrap_or(0).max(0);
            (limit, offset)
        };

        // Convert -1 to i64::MAX for unlimited queries
        let limit = if limit == -1 { i64::MAX } else { limit };
        (limit, offset)
    }

    fn get_page(&self, default: i64) -> i64 {
        self.page.map_or_else(
            || {
                if let (Some(limit), Some(offset)) = (self.limit, self.offset) {
                    if limit > 0 {
                        (offset / limit) + 1
                    } else {
                        default
                    }
                } else if let Some(limit) = self.limit {
                    let offset = self.offset.unwrap_or(0);
                    if limit > 0 {
                        (offset / limit) + 1
                    } else {
                        default
                    }
                } else if self.per_page.is_some() {
                    self.page.unwrap_or(default).max(1)
                } else {
                    default
                }
            },
            |page| page.max(1),
        )
    }

    fn get_per_page(&self, default: i64, max_limit: i64) -> i64 {
        self.per_page.map_or_else(
            || {
                self.limit.map_or_else(
                    || default.clamp(1, max_limit),
                    |limit| {
                        if limit == -1 || limit == i64::MAX {
                            i64::MAX
                        } else {
                            limit.clamp(1, max_limit)
                        }
                    },
                )
            },
            |per_page| {
                if per_page == -1 {
                    i64::MAX
                } else {
                    per_page.clamp(1, max_limit)
                }
            },
        )
    }
}
