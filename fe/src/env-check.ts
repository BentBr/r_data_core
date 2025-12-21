// Environment Variables Check
// This utility helps verify that Vite is properly reading environment variables

// Type definition for runtime configuration injected by docker-entrypoint.sh
declare global {
    interface Window {
        __APP_CONFIG__?: {
            VITE_API_BASE_URL?: string
            VITE_ADMIN_BASE_URL?: string
            VITE_DEV_MODE?: string
            VITE_ENABLE_API_LOGGING?: string
            VITE_DEFAULT_PAGE_SIZE?: string
            VITE_MAX_PAGE_SIZE?: string
            VITE_TOKEN_REFRESH_BUFFER?: string
        }
    }
}

/**
 * Get a configuration value with fallback priority:
 * 1. Runtime config (window.__APP_CONFIG__) - highest priority
 * 2. Build-time config (import.meta.env) - for dev/build
 * 3. Fallback value - if provided
 */
function getConfigValue(key: string, fallback?: string): string | undefined {
    // Check runtime config first (injected at container startup)
    if (
        typeof window !== 'undefined' &&
        window.__APP_CONFIG__?.[key as keyof typeof window.__APP_CONFIG__]
    ) {
        return window.__APP_CONFIG__[key as keyof typeof window.__APP_CONFIG__]
    }

    // Fall back to build-time Vite env vars
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const viteValue = (import.meta.env as any)[key]
    if (viteValue && typeof viteValue === 'string' && viteValue.trim() !== '') {
        return viteValue
    }

    // Return fallback if provided
    return fallback
}

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
 * Priority:
 * 1. Runtime config (window.__APP_CONFIG__.VITE_API_BASE_URL) - from container env vars
 * 2. Build-time config (import.meta.env.VITE_API_BASE_URL) - for dev/build
 * 3. In development, use empty string for relative URLs (proxied by Vite)
 * 4. In production, fall back to current origin (backwards compatibility)
 */
function getApiBaseUrl(): string {
    // Check runtime config first (highest priority)
    const runtimeUrl = getConfigValue('VITE_API_BASE_URL')
    if (runtimeUrl && runtimeUrl.trim() !== '') {
        return runtimeUrl.trim()
    }

    // In development, use empty string for relative URLs (Vite proxy handles it)
    if (import.meta.env.DEV) {
        return ''
    }

    // In production, fall back to current origin if env var not set (backwards compatibility)
    // Note: This is not ideal - VITE_API_BASE_URL should be set at runtime
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
    get adminBaseUrl() {
        return getConfigValue('VITE_ADMIN_BASE_URL') ?? import.meta.env.VITE_ADMIN_BASE_URL
    },
    get devMode() {
        const value = getConfigValue('VITE_DEV_MODE') ?? import.meta.env.VITE_DEV_MODE
        return value === 'true'
    },
    get enableApiLogging() {
        const value =
            getConfigValue('VITE_ENABLE_API_LOGGING') ?? import.meta.env.VITE_ENABLE_API_LOGGING
        return value === 'true'
    },
    get defaultPageSize() {
        const value =
            getConfigValue('VITE_DEFAULT_PAGE_SIZE') ?? import.meta.env.VITE_DEFAULT_PAGE_SIZE
        return parseInt(value ?? '100', 10)
    },
    get maxPageSize() {
        const value = getConfigValue('VITE_MAX_PAGE_SIZE') ?? import.meta.env.VITE_MAX_PAGE_SIZE
        return parseInt(value ?? '100', 10)
    },
    get tokenRefreshBuffer() {
        const value =
            getConfigValue('VITE_TOKEN_REFRESH_BUFFER') ?? import.meta.env.VITE_TOKEN_REFRESH_BUFFER
        return parseInt(value ?? '5', 10)
    },
    isProduction: import.meta.env.PROD,
    isDevelopment: import.meta.env.DEV,
}
