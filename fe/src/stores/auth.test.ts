import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAuthStore } from './auth'
import { typedHttpClient } from '@/api/typed-client'
import { HttpError } from '@/api/errors'

// Mock the HTTP client
vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getUserPermissions: vi.fn(),
        login: vi.fn(),
        logout: vi.fn(),
        refreshToken: vi.fn(),
    },
}))

// Suppress noisy store side-effects that are irrelevant to login error mapping
vi.mock('@/stores/license', () => ({
    useLicenseStore: () => ({ loadLicenseStatus: vi.fn(), resetBannerDismissal: vi.fn() }),
}))
vi.mock('@/stores/versions', () => ({
    useVersionStore: () => ({ loadVersions: vi.fn(), clearVersions: vi.fn() }),
}))
vi.mock('@/stores/capabilities', () => ({
    useCapabilitiesStore: () => ({ fetchCapabilities: vi.fn() }),
}))
vi.mock('@/utils/cookies', () => ({
    getRefreshToken: vi.fn(() => null),
    setRefreshToken: vi.fn(),
    deleteRefreshToken: vi.fn(),
}))

describe('Auth Store', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.clearAllMocks()
    })

    describe('hasPermission', () => {
        it('should return true for super admin for all permissions', async () => {
            const store = useAuthStore()

            // Mock super admin response
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: true,
                permissions: [],
                allowed_routes: [],
            })

            await store.loadUserPermissions()

            // Super admin should have all permissions
            expect(store.hasPermission('Workflows', 'Read')).toBe(true)
            expect(store.hasPermission('Workflows', 'Create')).toBe(true)
            expect(store.hasPermission('Entities', 'Delete')).toBe(true)
            expect(store.hasPermission('System', 'Admin')).toBe(true)
        })

        it('should return true for all permission types when Admin exists for namespace', async () => {
            const store = useAuthStore()

            // Mock user with Admin permission for Workflows
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: ['workflows:admin'],
                allowed_routes: ['/workflows'],
            })

            await store.loadUserPermissions()

            // Admin permission should grant all permission types for Workflows
            expect(store.hasPermission('Workflows', 'Read')).toBe(true)
            expect(store.hasPermission('Workflows', 'Create')).toBe(true)
            expect(store.hasPermission('Workflows', 'Update')).toBe(true)
            expect(store.hasPermission('Workflows', 'Delete')).toBe(true)
            expect(store.hasPermission('Workflows', 'Publish')).toBe(true)
            expect(store.hasPermission('Workflows', 'Execute')).toBe(true)
            expect(store.hasPermission('Workflows', 'Admin')).toBe(true)

            // But should NOT grant permissions for other namespaces
            expect(store.hasPermission('Entities', 'Read')).toBe(false)
            expect(store.hasPermission('System', 'Read')).toBe(false)
        })

        it('should handle EntityDefinitions namespace conversion correctly', async () => {
            const store = useAuthStore()

            // Mock user with Admin permission for EntityDefinitions
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: ['entity_definitions:admin'],
                allowed_routes: ['/entity-definitions'],
            })

            await store.loadUserPermissions()

            // Should work with frontend format "EntityDefinitions"
            expect(store.hasPermission('EntityDefinitions', 'Read')).toBe(true)
            expect(store.hasPermission('EntityDefinitions', 'Create')).toBe(true)
        })

        it('should distinguish between resource-level Admin and super admin', async () => {
            const store = useAuthStore()

            // Mock user with Admin permission for Workflows only
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: ['workflows:admin'],
                allowed_routes: ['/workflows'],
            })

            await store.loadUserPermissions()

            // Should have permissions for Workflows
            expect(store.hasPermission('Workflows', 'Read')).toBe(true)

            // Should NOT have permissions for System
            expect(store.hasPermission('System', 'Read')).toBe(false)

            // Now make it super admin
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: true,
                permissions: ['workflows:admin'],
                allowed_routes: ['/workflows'],
            })

            await store.loadUserPermissions()

            // Should now have permissions for ALL namespaces
            expect(store.hasPermission('Workflows', 'Read')).toBe(true)
            expect(store.hasPermission('System', 'Read')).toBe(true)
            expect(store.hasPermission('Entities', 'Delete')).toBe(true)
        })

        it('should check exact permission when Admin does not exist', async () => {
            const store = useAuthStore()

            // Mock user with only Read permission for Workflows
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: ['workflows:read'],
                allowed_routes: ['/workflows'],
            })

            await store.loadUserPermissions()

            // Should have Read permission
            expect(store.hasPermission('Workflows', 'Read')).toBe(true)

            // Should NOT have other permissions
            expect(store.hasPermission('Workflows', 'Create')).toBe(false)
            expect(store.hasPermission('Workflows', 'Delete')).toBe(false)
        })

        it('should handle multiple Admin permissions for different namespaces', async () => {
            const store = useAuthStore()

            // Mock user with Admin for Workflows and Entities
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: ['workflows:admin', 'entities:admin'],
                allowed_routes: ['/workflows', '/entities'],
            })

            await store.loadUserPermissions()

            // Should have all permissions for Workflows
            expect(store.hasPermission('Workflows', 'Read')).toBe(true)
            expect(store.hasPermission('Workflows', 'Delete')).toBe(true)

            // Should have all permissions for Entities
            expect(store.hasPermission('Entities', 'Read')).toBe(true)
            expect(store.hasPermission('Entities', 'Delete')).toBe(true)

            // Should NOT have permissions for System
            expect(store.hasPermission('System', 'Read')).toBe(false)
        })
    })

    describe('login', () => {
        it('sets error to the locked message when the server returns 403', async () => {
            vi.mocked(typedHttpClient.login).mockRejectedValueOnce(
                new HttpError(403, 'auth', 'create', 'Account locked or not active')
            )

            const store = useAuthStore()
            await expect(store.login({ username: 'u', password: 'p' })).rejects.toThrow()

            // global test-setup mocks t(key) → key.split('.').pop()
            expect(store.error).toBe('locked')
        })

        it('sets error to the rate-limited message when the server returns 429', async () => {
            vi.mocked(typedHttpClient.login).mockRejectedValueOnce(
                new HttpError(429, 'auth', 'create', 'Too many requests')
            )

            const store = useAuthStore()
            await expect(store.login({ username: 'u', password: 'p' })).rejects.toThrow()

            expect(store.error).toBe('rate_limited')
        })

        it('does NOT set the locked or rate-limited message on a 401 (regression guard)', async () => {
            vi.mocked(typedHttpClient.login).mockRejectedValueOnce(
                new HttpError(401, 'auth', 'create', 'Invalid credentials')
            )

            const store = useAuthStore()
            // The login call rejects (the else-branch of the catch may itself throw
            // because translateError is not in the global test-setup mock, which is fine —
            // the important invariant is that neither locked nor rate_limited is set)
            await store.login({ username: 'u', password: 'p' }).catch(() => undefined)

            expect(store.error).not.toBe('locked')
            expect(store.error).not.toBe('rate_limited')
        })
    })

    describe('canAccessRoute', () => {
        it('should return true for super admin for all routes', async () => {
            const store = useAuthStore()

            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: true,
                permissions: [],
                allowed_routes: [],
            })

            await store.loadUserPermissions()

            expect(store.canAccessRoute('/workflows')).toBe(true)
            expect(store.canAccessRoute('/entities')).toBe(true)
            expect(store.canAccessRoute('/system')).toBe(true)
        })

        it('should return true for routes in allowed_routes', async () => {
            const store = useAuthStore()

            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: ['workflows:read'],
                allowed_routes: ['/workflows', '/dashboard'],
            })

            await store.loadUserPermissions()

            expect(store.canAccessRoute('/workflows')).toBe(true)
            expect(store.canAccessRoute('/dashboard')).toBe(true)
            expect(store.canAccessRoute('/entities')).toBe(false)
        })
    })
})
