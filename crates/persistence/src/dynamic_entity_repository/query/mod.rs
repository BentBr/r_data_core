mod fetch;
mod lookup;

pub use fetch::{get_all_by_type_impl, get_by_type_impl};
pub use lookup::{
    count_children_impl, count_entities_impl, delete_by_type_impl, find_one_by_filters_impl,
    get_by_uuid_any_type_impl, has_children_impl, query_by_parent_impl, query_by_path_impl,
};

use sqlx::PgPool;
use uuid::Uuid;

/// Check if an error is the "cached plan must not change result type" error
/// This occurs when cached plan types change (aka an entity definition changes) and not every connection has gotten the updated cache yet
pub(super) fn is_cached_plan_error(err: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(db_err) = err {
        // PostgreSQL error code 0A000 = feature_not_supported
        // This specific message occurs when cached plan types change
        db_err.code().is_some_and(|c| c == "0A000")
            && db_err
                .message()
                .contains("cached plan must not change result type")
    } else {
        false
    }
}

/// Query bind value types we support
#[derive(Clone)]
pub(super) enum QueryBind<'a> {
    Uuid(Uuid),
    I64(i64),
    String(&'a str),
}

/// Execute a query with binds
pub(super) async fn execute_query<'q>(
    pool: &PgPool,
    query: &'q str,
    binds: &[QueryBind<'q>],
) -> std::result::Result<Vec<sqlx::postgres::PgRow>, sqlx::Error> {
    let mut q = sqlx::query(query);
    for bind in binds {
        q = match bind {
            QueryBind::Uuid(v) => q.bind(*v),
            QueryBind::I64(v) => q.bind(*v),
            QueryBind::String(v) => q.bind(*v),
        };
    }
    q.fetch_all(pool).await
}

/// Execute a query with binds (optional result)
pub(super) async fn execute_query_optional<'q>(
    pool: &PgPool,
    query: &'q str,
    binds: &[QueryBind<'q>],
) -> std::result::Result<Option<sqlx::postgres::PgRow>, sqlx::Error> {
    let mut q = sqlx::query(query);
    for bind in binds {
        q = match bind {
            QueryBind::Uuid(v) => q.bind(*v),
            QueryBind::I64(v) => q.bind(*v),
            QueryBind::String(v) => q.bind(*v),
        };
    }
    q.fetch_optional(pool).await
}

/// Execute a query with retry logic for cached plan errors.
/// On the first "cached plan must not change result type" error,
/// we run DISCARD PLANS to clear the statement cache and retry once.
pub(super) async fn fetch_all_with_retry<'q>(
    pool: &PgPool,
    query: &'q str,
    binds: Vec<QueryBind<'q>>,
) -> std::result::Result<Vec<sqlx::postgres::PgRow>, sqlx::Error> {
    let result = execute_query(pool, query, &binds).await;

    match result {
        Err(ref e) if is_cached_plan_error(e) => {
            log::warn!("Cached plan error detected, discarding plans and retrying query");

            // Clear cached plans on this connection
            sqlx::query("DISCARD PLANS").execute(pool).await?;

            // Retry the query
            execute_query(pool, query, &binds).await
        }
        other => other,
    }
}

/// Execute a query with retry logic for cached plan errors (single row).
pub(super) async fn fetch_optional_with_retry<'q>(
    pool: &PgPool,
    query: &'q str,
    binds: Vec<QueryBind<'q>>,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, sqlx::Error> {
    let result = execute_query_optional(pool, query, &binds).await;

    match result {
        Err(ref e) if is_cached_plan_error(e) => {
            log::warn!("Cached plan error detected, discarding plans and retrying query");

            // Clear cached plans on this connection
            sqlx::query("DISCARD PLANS").execute(pool).await?;

            // Retry the query
            execute_query_optional(pool, query, &binds).await
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_cached_plan_error_returns_false_for_non_database_errors() {
        // Protocol error - not a database error
        let err = sqlx::Error::Protocol("some protocol error".to_string());
        assert!(!is_cached_plan_error(&err));

        // Row not found error
        let err = sqlx::Error::RowNotFound;
        assert!(!is_cached_plan_error(&err));

        // Configuration error
        let err = sqlx::Error::Configuration("bad config".into());
        assert!(!is_cached_plan_error(&err));
    }

    #[test]
    fn query_bind_enum_is_clone() {
        // Verify QueryBind implements Clone (required for retry logic)
        let uuid_bind = QueryBind::Uuid(uuid::Uuid::nil());
        let _cloned = uuid_bind.clone();

        let i64_bind = QueryBind::I64(42);
        let _cloned = i64_bind.clone();

        let str_bind = QueryBind::String("test");
        let _cloned = str_bind.clone();
    }
}
