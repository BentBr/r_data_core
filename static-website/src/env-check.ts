type EnvConfig = {
    baseUrl: string
    siteName: string
    demoUrl: string
    isProduction: boolean
    isDevelopment: boolean
}

function getEnv(name: string): string {
    const value = import.meta.env[name]
    if (value === undefined || value === null) {
        return ''
    }
    return String(value)
}

export function checkEnvironmentVariables(): EnvConfig {
    const baseUrl = getEnv('VITE_BASE_URL')
    const siteName = getEnv('VITE_SITE_NAME')
    const demoUrl = getEnv('R_DATA_CORE_DEMO_URL')

    const envVars = {
        VITE_BASE_URL: baseUrl,
        VITE_SITE_NAME: siteName,
        R_DATA_CORE_DEMO_URL: demoUrl,
        MODE: import.meta.env.MODE,
        DEV: import.meta.env.DEV,
        PROD: import.meta.env.PROD,
    }

    if (import.meta.env.DEV) {
        console.group('🔧 Environment Variables')
        console.table(envVars)
        console.groupEnd()
    }

    return {
        baseUrl,
        siteName,
        demoUrl,
        isProduction: import.meta.env.PROD,
        isDevelopment: import.meta.env.DEV,
    }
}

export const env = {
    get baseUrl() {
        return (
            getEnv('VITE_BASE_URL') || (typeof window !== 'undefined' ? window.location.origin : '')
        )
    },
    get siteName() {
        return getEnv('VITE_SITE_NAME') || 'RDataCore'
    },
    get demoUrl() {
        return getEnv('R_DATA_CORE_DEMO_URL')
    },
    isProduction: import.meta.env.PROD,
    isDevelopment: import.meta.env.DEV,
}
