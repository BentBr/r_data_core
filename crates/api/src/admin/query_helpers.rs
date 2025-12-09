#![deny(clippy::all, clippy::pedantic, clippy::nursery, warnings)]

use crate::query::StandardQuery;
use r_data_core_services::ListQueryParams;

/// Convert `StandardQuery` to `ListQueryParams` for service layer.
#[must_use]
pub fn to_list_query_params(query: &StandardQuery) -> ListQueryParams {
    ListQueryParams {
        page: query.pagination.page,
        per_page: query.pagination.per_page,
        limit: query.pagination.limit,
        offset: query.pagination.offset,
        sort_by: query.sorting.sort_by.clone(),
        sort_order: query.sorting.sort_order.clone(),
    }
}
