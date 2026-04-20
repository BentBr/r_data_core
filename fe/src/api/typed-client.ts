import { TypedHttpClient } from './clients/index'

export { ValidationError } from './http-client'

// Re-export generated + Zod-derived types so components can `import type { ... } from '@/api/typed-client'`.
export type * from '@/types/schemas'

// Singleton — the FE's single entry point for BE calls.
export const typedHttpClient = new TypedHttpClient()
