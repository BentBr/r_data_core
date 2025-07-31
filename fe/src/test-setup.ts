/**
 * Jest test setup file
 */

// Mock environment variables for testing
const mockEnv = {
    VITE_API_BASE_URL: 'http://localhost:8888',
    VITE_ADMIN_BASE_URL: 'http://localhost:8888',
    VITE_DEV_MODE: 'true',
    VITE_ENABLE_API_LOGGING: 'false',
    VITE_DEFAULT_PAGE_SIZE: '20',
    VITE_MAX_PAGE_SIZE: '100',
    VITE_TOKEN_REFRESH_BUFFER: '5',
    MODE: 'development',
    DEV: true,
    PROD: false,
}

// Mock import.meta.env
Object.defineProperty(global, 'import', {
    value: {
        meta: {
            env: mockEnv,
        },
    },
    writable: true,
})

// Fix document.cookie to be writable in jsdom
Object.defineProperty(window.document, 'cookie', {
    writable: true,
    configurable: true,
    value: '',
})

// Mock console methods to reduce noise in tests
global.console = {
    ...console,
    log: jest.fn(),
    warn: jest.fn(),
    error: jest.fn(),
    info: jest.fn(),
    debug: jest.fn(),
}
