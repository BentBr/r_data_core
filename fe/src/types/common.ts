/**
 * Common type definitions used across the application
 */

/**
 * Table row item interface - base structure for all table items
 * @template T - Optional type parameter for specific row data structure
 */
export type TableRow<T extends Record<string, unknown> = Record<string, unknown>> = {
    uuid: string
} & T

/**
 * Pagination metadata structure
 */
export interface PaginationMeta {
    total: number
    page: number
    per_page: number
    total_pages: number
    has_previous: boolean
    has_next: boolean
}

/**
 * Table header definition
 */
export interface TableHeader {
    title: string
    key: string
    sortable?: boolean
    align?: 'start' | 'center' | 'end'
    width?: string
}

/**
 * Table action definition
 */
export interface TableAction {
    icon: string
    color?: string
    disabled?: boolean
    loading?: boolean
    onClick?: (item: TableRow) => void
}
