import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// Mock the auth store - this must be hoisted before any imports
const createMockAuthStore = (overrides = {}) => {
    const defaultStore = {
        isAuthenticated: true,
        isTokenExpired: false,
        canAccessRoute: vi.fn((route: string) => {
            // Default: only allow dashboard
            return route === '/dashboard'
        }),
        checkAuthStatus: vi.fn().mockResolvedValue(undefined),
        logout: vi.fn().mockResolvedValue(undefined),
        ...overrides,
    }
    return defaultStore
}

let mockAuthStore: ReturnType<typeof createMockAuthStore>

// Mock must be defined before importing router
vi.mock('@/stores/auth', () => ({
    useAuthStore: () => mockAuthStore,
}))

// Import router after mock is set up
import router from './index'

describe('Router Permission Guards', () => {
    beforeEach(async () => {
        // Initialize mock store with default values
        mockAuthStore = createMockAuthStore()
        vi.clearAllMocks()

        // Set initial route to dashboard
        await router.push('/dashboard')
        await router.isReady()
    })

    afterEach(async () => {
        // Clean up - reset to dashboard
        if (router) {
            await router.push('/dashboard')
        }
    })

    it('should redirect to dashboard when user lacks permission for a route', async () => {
        // Setup: User is authenticated but doesn't have permission for /permissions
        mockAuthStore.isAuthenticated = true
        mockAuthStore.isTokenExpired = false
        mockAuthStore.canAccessRoute = vi.fn((route: string) => {
            // Only allow dashboard, not permissions
            return route === '/dashboard'
        })

        // Navigate to a route the user doesn't have permission for
        await router.push('/permissions')

        // Should be redirected to dashboard
        expect(router.currentRoute.value.name).toBe('Dashboard')
        expect(router.currentRoute.value.path).toBe('/dashboard')
        expect(mockAuthStore.canAccessRoute).toHaveBeenCalledWith('/permissions')
    })

    it('should redirect to dashboard when user lacks permission for /api-keys', async () => {
        // Setup: User is authenticated but doesn't have permission for /api-keys
        mockAuthStore.isAuthenticated = true
        mockAuthStore.isTokenExpired = false
        mockAuthStore.canAccessRoute = vi.fn((route: string) => {
            // Only allow dashboard
            return route === '/dashboard'
        })

        // Navigate to /api-keys
        await router.push('/api-keys')

        // Should be redirected to dashboard
        expect(router.currentRoute.value.name).toBe('Dashboard')
        expect(router.currentRoute.value.path).toBe('/dashboard')
        expect(mockAuthStore.canAccessRoute).toHaveBeenCalledWith('/api-keys')
    })

    it('should redirect to dashboard when user lacks permission for /entity-definitions', async () => {
        // Setup: User is authenticated but doesn't have permission
        mockAuthStore.isAuthenticated = true
        mockAuthStore.isTokenExpired = false
        mockAuthStore.canAccessRoute = vi.fn((route: string) => {
            return route === '/dashboard'
        })

        // Navigate to /entity-definitions
        await router.push('/entity-definitions')

        // Should be redirected to dashboard
        expect(router.currentRoute.value.name).toBe('Dashboard')
        expect(router.currentRoute.value.path).toBe('/dashboard')
        expect(mockAuthStore.canAccessRoute).toHaveBeenCalledWith('/entity-definitions')
    })

    it('should redirect to dashboard when user lacks permission for /system', async () => {
        // Setup: User is authenticated but doesn't have permission
        mockAuthStore.isAuthenticated = true
        mockAuthStore.isTokenExpired = false
        mockAuthStore.canAccessRoute = vi.fn((route: string) => {
            return route === '/dashboard'
        })

        // Navigate to /system
        await router.push('/system')

        // Should be redirected to dashboard
        expect(router.currentRoute.value.name).toBe('Dashboard')
        expect(router.currentRoute.value.path).toBe('/dashboard')
        expect(mockAuthStore.canAccessRoute).toHaveBeenCalledWith('/system')
    })

    it('should allow access when user has permission for the route', async () => {
        // Setup: User is authenticated and has permission for /workflows
        mockAuthStore.isAuthenticated = true
        mockAuthStore.isTokenExpired = false
        mockAuthStore.canAccessRoute = vi.fn((route: string) => {
            // Allow dashboard and workflows
            return route === '/dashboard' || route === '/workflows'
        })

        // Navigate to /workflows
        await router.push('/workflows')

        // Should be able to access the route
        expect(router.currentRoute.value.name).toBe('Workflows')
        expect(router.currentRoute.value.path).toBe('/workflows')
        expect(mockAuthStore.canAccessRoute).toHaveBeenCalledWith('/workflows')
    })

    it('should always allow access to dashboard if authenticated', async () => {
        // Setup: User is authenticated
        mockAuthStore.isAuthenticated = true
        mockAuthStore.isTokenExpired = false
        mockAuthStore.canAccessRoute = vi.fn((route: string) => {
            // Only allow dashboard
            return route === '/dashboard'
        })

        // Navigate to dashboard
        await router.push('/dashboard')

        // Should be able to access dashboard
        expect(router.currentRoute.value.name).toBe('Dashboard')
        expect(router.currentRoute.value.path).toBe('/dashboard')
        // Note: canAccessRoute should not be called for dashboard (per router logic)
    })

    it('should redirect to dashboard when user lacks permission for /entities', async () => {
        // Setup: User is authenticated but doesn't have permission
        mockAuthStore.isAuthenticated = true
        mockAuthStore.isTokenExpired = false
        mockAuthStore.canAccessRoute = vi.fn((route: string) => {
            return route === '/dashboard'
        })

        // Navigate to /entities
        await router.push('/entities')

        // Should be redirected to dashboard
        expect(router.currentRoute.value.name).toBe('Dashboard')
        expect(router.currentRoute.value.path).toBe('/dashboard')
        expect(mockAuthStore.canAccessRoute).toHaveBeenCalledWith('/entities')
    })
})
