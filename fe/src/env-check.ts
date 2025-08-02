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

// Type-safe environment variable getters
export const env = {
    apiBaseUrl: import.meta.env.VITE_API_BASE_URL,
    adminBaseUrl: import.meta.env.VITE_ADMIN_BASE_URL,
    devMode: import.meta.env.VITE_DEV_MODE === 'true',
    enableApiLogging: import.meta.env.VITE_ENABLE_API_LOGGING === 'true',
    defaultPageSize: parseInt(import.meta.env.VITE_DEFAULT_PAGE_SIZE || '100', 10),
    maxPageSize: parseInt(import.meta.env.VITE_MAX_PAGE_SIZE || '100', 10),
    tokenRefreshBuffer: parseInt(import.meta.env.VITE_TOKEN_REFRESH_BUFFER || '5', 10),
    isProduction: import.meta.env.PROD,
    isDevelopment: import.meta.env.DEV,
}

// Feature flags
export const features = {
    apiKeyManagement: import.meta.env.VITE_ENABLE_API_KEY_MANAGEMENT !== 'false',
    userManagement: import.meta.env.VITE_ENABLE_USER_MANAGEMENT !== 'false',
    systemMetrics: import.meta.env.VITE_ENABLE_SYSTEM_METRICS !== 'false',
}
