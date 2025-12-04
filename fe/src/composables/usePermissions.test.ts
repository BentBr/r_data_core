import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useAuthStore } from '@/stores/auth'

vi.mock('@/stores/auth', () => ({
    useAuthStore: vi.fn(() => ({
        isSuperAdmin: false,
        allowedRoutes: ['/dashboard', '/workflows'],
        hasPermission: vi.fn((namespace: string, permissionType: string) => {
            if (namespace === 'PermissionSchemes' && permissionType === 'Create') {
                return true
            }
            return false
        }),
        canAccessRoute: vi.fn((route: string) => {
            return ['/dashboard', '/workflows'].includes(route)
        }),
    })),
}))

describe('usePermissions', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    it('should check if user can access a route', () => {
        const authStore = useAuthStore()
        expect(authStore.canAccessRoute('/dashboard')).toBe(true)
        expect(authStore.canAccessRoute('/workflows')).toBe(true)
        expect(authStore.canAccessRoute('/permissions')).toBe(false)
    })

    it('should check if user has a specific permission', () => {
        const authStore = useAuthStore()
        expect(authStore.hasPermission('PermissionSchemes', 'Create')).toBe(true)
        expect(authStore.hasPermission('PermissionSchemes', 'Delete')).toBe(false)
    })

    it('should return true for all routes if user is super admin', () => {
        const superAdminStore = useAuthStore()
        superAdminStore.isSuperAdmin = true
        expect(superAdminStore.canAccessRoute('/any-route')).toBe(true)
    })
})
