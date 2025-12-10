// Environment Variables Check
// This utility helps verify that Vite is properly reading environment variables

export function checkEnvironmentVariables() {
    const envVars = {
        VITE_BASE_URL: import.meta.env.VITE_BASE_URL,
        VITE_SITE_NAME: import.meta.env.VITE_SITE_NAME,
        VITE_R_DATA_CORE_DEMO_URL: import.meta.env.VITE_R_DATA_CORE_DEMO_URL,
        VITE_API_DOCS_URL: import.meta.env.VITE_API_DOCS_URL,
        VITE_ADMIN_API_DOCS_URL: import.meta.env.VITE_ADMIN_API_DOCS_URL,
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
 * Get the base URL with proper fallback
 * In production, uses env var or falls back to window.location.origin
 */
export function getBaseUrl(): string {
    const envUrl = import.meta.env.VITE_BASE_URL

    // If env var is explicitly set, use it
    if (envUrl && envUrl.trim() !== '') {
        return envUrl.trim()
    }

    // In production, fall back to current origin if env var not set
    if (typeof window !== 'undefined') {
        return window.location.origin
    }

    // Server-side rendering fallback
    return ''
}

// Type-safe environment variable getters
// Using direct property access instead of getters to avoid reactive loops
export const env = {
    get baseUrl() {
        return getBaseUrl()
    },
    siteName: import.meta.env.VITE_SITE_NAME ?? 'RDataCore',
    demoUrl: import.meta.env.VITE_R_DATA_CORE_DEMO_URL ?? '',
    apiDocsUrl: import.meta.env.VITE_API_DOCS_URL ?? '',
    adminApiDocsUrl: import.meta.env.VITE_ADMIN_API_DOCS_URL ?? '',
    isProduction: import.meta.env.PROD,
    isDevelopment: import.meta.env.DEV,
}
