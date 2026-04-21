import type { PaginationQuery } from '@/types/generated/PaginationQuery'
import type { SortingQuery } from '@/types/generated/SortingQuery'

/**
 * Serialize typed BE query shapes (PaginationQuery + SortingQuery) into a URL query string.
 * Null fields are omitted so the emitted URL only carries the parameters the caller set.
 */
export function buildListQueryString(
    pagination: PaginationQuery,
    sorting?: SortingQuery | null
): string {
    const params = new URLSearchParams()
    if (pagination.page != null) params.set('page', String(pagination.page))
    if (pagination.per_page != null) params.set('per_page', String(pagination.per_page))
    if (pagination.limit != null) params.set('limit', String(pagination.limit))
    if (pagination.offset != null) params.set('offset', String(pagination.offset))
    if (sorting?.sort_by != null && sorting.sort_order != null) {
        params.set('sort_by', sorting.sort_by)
        params.set('sort_order', sorting.sort_order)
    }
    const qs = params.toString()
    return qs ? `?${qs}` : ''
}
