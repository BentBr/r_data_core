// Re-export for backward compatibility
// New code should import from './clients/index' or specific client files
export { TypedHttpClient } from './clients/index'
export { ValidationError } from './http-client'
export { HttpClient as TypedHttpClientBase } from './http-client'
export type { ApiResponse } from './http-client'

// Re-export all types for convenience
export type * from '@/types/schemas'

// Create and export singleton instance
import { TypedHttpClient } from './clients/index'
export const typedHttpClient = new TypedHttpClient()
export type { TypedHttpClient }
