// Custom error class for HTTP errors with status code, namespace, and action
export type HttpAction = 'create' | 'read' | 'update' | 'delete' | 'unknown'

export class HttpError extends Error {
    statusCode: number
    namespace: string
    action: HttpAction
    originalMessage?: string

    constructor(
        statusCode: number,
        namespace: string,
        action: HttpAction,
        message: string,
        originalMessage?: string
    ) {
        super(message) // Clean message without HTTP code
        this.name = 'HttpError'
        this.statusCode = statusCode
        this.namespace = namespace
        this.action = action
        this.originalMessage = originalMessage ?? message
    }
}

/**
 * Extract HTTP action from HTTP method
 */
export function extractActionFromMethod(method?: string): HttpAction {
    const upperMethod = (method ?? 'GET').toUpperCase()
    switch (upperMethod) {
        case 'POST':
            return 'create'
        case 'GET':
            return 'read'
        case 'PUT':
        case 'PATCH':
            return 'update'
        case 'DELETE':
            return 'delete'
        default:
            return 'unknown' // Default fallback
    }
}

/**
 * Extract namespace from API endpoint
 * Examples:
 * - /admin/api/v1/users -> user
 * - /admin/api/v1/entity-definitions -> entity_definition
 * - /admin/api/v1/api-keys -> api_key
 * - /admin/api/v1/workflows -> workflow
 * - /admin/api/v1/roles -> role
 * - /api/v1/entities/my_type/123 -> entity
 * - /api/v1/MyEntityType -> entity (dynamic entity type endpoints)
 */
export function extractNamespaceFromEndpoint(endpoint: string): string {
    // Remove query parameters
    const path = endpoint.split('?')[0]

    // Split path into segments
    const segments = path.split('/').filter(s => s)

    // Check if this is an admin API endpoint
    const isAdminApi = segments.includes('admin')

    // Skip common prefixes: admin, api, v1
    const skipPrefixes = ['admin', 'api', 'v1']
    let resourceSegment: string | undefined

    // Find the first segment that's not a skip prefix
    for (const segment of segments) {
        if (!skipPrefixes.includes(segment)) {
            resourceSegment = segment
            break
        }
    }

    if (resourceSegment) {
        let namespace = resourceSegment.replace(/-/g, '_')
        // Handle special cases
        if (namespace === 'api_keys') {
            namespace = 'api_key'
        } else if (namespace === 'entities') {
            // For /api/v1/entities/... endpoints, return 'entity' (singular)
            namespace = 'entity'
        } else if (namespace === 'entity_definitions') {
            namespace = 'entity_definition'
        } else if (namespace.endsWith('_definitions')) {
            // Handle entity-definitions, workflow-definitions, etc.
            namespace = namespace.replace('_definitions', '_definition')
        } else if (namespace.endsWith('s') && namespace.length > 1) {
            // Convert plural to singular (users -> user, roles -> role, workflows -> workflow)
            // But keep special cases like 'api_keys' which is already handled above
            namespace = namespace.slice(0, -1)
        }

        // Known admin API namespaces - these have specific translations
        const knownNamespaces = [
            'user',
            'role',
            'api_key',
            'entity_definition',
            'workflow',
            'entity',
            'system',
            'auth',
            'dsl',
        ]

        // For public API endpoints (non-admin), if the namespace is not a known one,
        // it's likely a dynamic entity type (e.g., /api/v1/Product, /api/v1/Puh)
        // In that case, use 'entity' as the namespace for translation purposes
        if (!isAdminApi && !knownNamespaces.includes(namespace)) {
            namespace = 'entity'
        }

        return namespace
    }
    return 'unknown' // Fallback namespace
}
