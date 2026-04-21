/**
 * Common UI-only type definitions used across the application.
 * Payload shapes from the backend live in `@/types/generated` and must not be duplicated here.
 */

/**
 * Table row item interface - base structure for all table items
 * @template T - Optional type parameter for specific row data structure
 */
export type TableRow<T extends Record<string, unknown> = Record<string, unknown>> = {
    uuid: string
} & T

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
