// Environment Variables Check
// This utility helps verify that Vite is properly reading environment variables

export function checkEnvironmentVariables() {
    const envVars = {
        VITE_API_BASE_URL: import.meta.env.VITE_API_BASE_URL,
        VITE_ADMIN_BASE_URL: import.meta.env.VITE_ADMIN_BASE_URL,
        VITE_DEV_MODE: import.meta.env.VITE_DEV_MODE,
        VITE_ENABLE_API_LOGGING: import.meta.env.VITE_ENABLE_API_LOGGING,
        VITE_DEFAULT_PAGE_SIZE: import.meta.env.VITE_DEFAULT_PAGE_SIZE,
        MODE: import.meta.env.MODE,
        DEV: import.meta.env.DEV,
        PROD: import.meta.env.PROD,
    }

    if (import.meta.env.DEV) {
        console.group('🔧 Environment Variables')
        console.table(envVars)
        console.groupEnd()
    }

    return envVars
}

/**
 * Get the API base URL with proper fallback
 * In development, uses empty string for relative URLs (proxied by Vite)
 * In production, uses env var or falls back to window.location.origin
 */
function getApiBaseUrl(): string {
    const envUrl = import.meta.env.VITE_API_BASE_URL

    // If env var is explicitly set, use it
    if (envUrl && envUrl.trim() !== '') {
        return envUrl.trim()
    }

    // In development, use empty string for relative URLs (Vite proxy handles it)
    if (import.meta.env.DEV) {
        return ''
    }

    // In production, fall back to current origin if env var not set
    if (typeof window !== 'undefined') {
        return window.location.origin
    }

    // Server-side rendering fallback
    return ''
}

/**
 * Build a full API URL from an endpoint path
 * @param endpoint - The API endpoint path (e.g., '/api/v1/users' or '/admin/api/v1/system/settings')
 * @returns The full URL
 */
export function buildApiUrl(endpoint: string): string {
    const baseUrl = getApiBaseUrl()
    const cleanEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`

    // If baseUrl is empty (dev mode with proxy), return relative URL
    if (baseUrl === '') {
        return cleanEndpoint
    }

    // Ensure baseUrl doesn't end with a slash
    const cleanBaseUrl = baseUrl.replace(/\/$/, '')
    return `${cleanBaseUrl}${cleanEndpoint}`
}

// Type-safe environment variable getters
export const env = {
    get apiBaseUrl() {
        return getApiBaseUrl()
    },
    adminBaseUrl: import.meta.env.VITE_ADMIN_BASE_URL,
    devMode: import.meta.env.VITE_DEV_MODE === 'true',
    enableApiLogging: import.meta.env.VITE_ENABLE_API_LOGGING === 'true',
    defaultPageSize: parseInt(import.meta.env.VITE_DEFAULT_PAGE_SIZE ?? '100', 10),
    maxPageSize: parseInt(import.meta.env.VITE_MAX_PAGE_SIZE ?? '100', 10),
    tokenRefreshBuffer: parseInt(import.meta.env.VITE_TOKEN_REFRESH_BUFFER ?? '5', 10),
    isProduction: import.meta.env.PROD,
    isDevelopment: import.meta.env.DEV,
}

// Feature flags
export const features = {
    apiKeyManagement: import.meta.env.VITE_ENABLE_API_KEY_MANAGEMENT !== 'false',
    userManagement: import.meta.env.VITE_ENABLE_USER_MANAGEMENT !== 'false',
    systemMetrics: import.meta.env.VITE_ENABLE_SYSTEM_METRICS !== 'false',
}
