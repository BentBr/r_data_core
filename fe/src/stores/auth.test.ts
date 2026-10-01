import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAuthStore } from './auth'
import { typedHttpClient } from '@/api/typed-client'
import { HttpError } from '@/api/errors'
import { getRefreshToken, setRefreshToken } from '@/utils/cookies'

// Mock the HTTP client
vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getUserPermissions: vi.fn(),
        login: vi.fn(),
        logout: vi.fn(),
        refreshToken: vi.fn(),
    },
}))

// Suppress noisy store side-effects that are irrelevant to login error mapping.
// Hoisted so the same spy instances are shared between the mock factory and
// the assertions below (an inline `() => ({ fn: vi.fn() })` factory would
// hand back a fresh, unobservable spy on every call).
const {
    mockLoadLicenseStatus,
    mockResetBannerDismissal,
    mockLoadVersions,
    mockClearVersions,
    mockFetchCapabilities,
} = vi.hoisted(() => ({
    mockLoadLicenseStatus: vi.fn(),
    mockResetBannerDismissal: vi.fn(),
    mockLoadVersions: vi.fn(),
    mockClearVersions: vi.fn(),
    mockFetchCapabilities: vi.fn(),
}))

vi.mock('@/stores/license', () => ({
    useLicenseStore: () => ({
        loadLicenseStatus: mockLoadLicenseStatus,
        resetBannerDismissal: mockResetBannerDismissal,
    }),
}))
vi.mock('@/stores/versions', () => ({
    useVersionStore: () => ({ loadVersions: mockLoadVersions, clearVersions: mockClearVersions }),
}))
vi.mock('@/stores/capabilities', () => ({
    useCapabilitiesStore: () => ({ fetchCapabilities: mockFetchCapabilities }),
}))
vi.mock('@/utils/cookies', () => ({
    getRefreshToken: vi.fn(() => null),
    setRefreshToken: vi.fn(),
    deleteRefreshToken: vi.fn(),
}))

/** Build a minimal, unsigned JWT carrying the given payload (good enough for atob/JSON.parse). */
function makeJwt(payload: Record<string, unknown>): string {
    const header = btoa(JSON.stringify({ alg: 'none', typ: 'JWT' }))
    const body = btoa(JSON.stringify(payload))
    return `${header}.${body}.signature`
}

describe('Auth Store', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.clearAllMocks()
        vi.mocked(getRefreshToken).mockReturnValue(null)
    })

    afterEach(() => {
        vi.useRealTimers()
        window.history.replaceState(null, '', '/')
        localStorage.clear()
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

    describe('login (success)', () => {
        it('establishes a full session: token, user, permissions and sibling stores', async () => {
            const futureExpiry = new Date(Date.now() + 10 * 60 * 1000).toISOString()
            const accessToken = makeJwt({ permissions: ['workflows:read'], is_super_admin: false })

            vi.mocked(typedHttpClient.login).mockResolvedValue({
                access_token: accessToken,
                refresh_token: 'refresh-123',
                user_uuid: 'user-uuid-1',
                username: 'alice',
                access_expires_at: futureExpiry,
                refresh_expires_at: futureExpiry,
                using_default_password: true,
            })
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: ['workflows:read'],
                allowed_routes: ['/workflows'],
            })

            const store = useAuthStore()
            await store.authReady
            await store.login({ username: 'alice', password: 'secret' })

            expect(store.token).toBe(accessToken)
            expect(store.user?.username).toBe('alice')
            expect(store.user?.uuid).toBe('user-uuid-1')
            expect(store.isAuthenticated).toBe(true)
            expect(store.isDefaultPasswordInUse).toBe(true)
            expect(store.permissions).toEqual(['workflows:read'])
            expect(setRefreshToken).toHaveBeenCalledWith('refresh-123', expect.any(Date))
            expect(mockLoadLicenseStatus).toHaveBeenCalled()
            expect(mockResetBannerDismissal).toHaveBeenCalled()
            expect(mockLoadVersions).toHaveBeenCalled()
            expect(mockFetchCapabilities).toHaveBeenCalled()
        })

        it('logs the user out immediately when access_expires_at is already in the past', async () => {
            const pastExpiry = new Date(Date.now() - 60 * 1000).toISOString()
            const accessToken = makeJwt({ permissions: [], is_super_admin: false })

            vi.mocked(typedHttpClient.login).mockResolvedValue({
                access_token: accessToken,
                refresh_token: 'refresh-123',
                user_uuid: 'user-uuid-1',
                username: 'alice',
                access_expires_at: pastExpiry,
                refresh_expires_at: pastExpiry,
                using_default_password: false,
            })
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: [],
                allowed_routes: [],
            })

            const store = useAuthStore()
            await store.authReady
            await store.login({ username: 'alice', password: 'secret' })

            // setupTokenRefresh schedules the logout as a fire-and-forget promise;
            // wait for the microtask queue to settle before asserting.
            await vi.waitFor(() => expect(store.token).toBeNull())
            expect(store.isAuthenticated).toBe(false)
        })

        it('schedules an automatic refresh that fires once the buffer window is reached', async () => {
            vi.useFakeTimers()
            const accessExpiresInMs = 10 * 60 * 1000 // 10 minutes; buffer defaults to 5 minutes
            const futureExpiry = new Date(Date.now() + accessExpiresInMs).toISOString()
            const accessToken = makeJwt({ permissions: [], is_super_admin: false })
            const refreshedToken = makeJwt({ sub: 'user-uuid-1', name: 'alice' })

            vi.mocked(typedHttpClient.login).mockResolvedValue({
                access_token: accessToken,
                refresh_token: 'refresh-123',
                user_uuid: 'user-uuid-1',
                username: 'alice',
                access_expires_at: futureExpiry,
                refresh_expires_at: futureExpiry,
                using_default_password: false,
            })
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: [],
                allowed_routes: [],
            })
            // The fake clock will have advanced by accessExpiresInMs by the time this
            // response is consumed, so the new expiry must be computed far enough
            // ahead (now) that it is still comfortably in the future at that point.
            const nextExpiry = new Date(Date.now() + accessExpiresInMs * 3).toISOString()
            vi.mocked(typedHttpClient.refreshToken).mockResolvedValue({
                access_token: refreshedToken,
                refresh_token: 'refresh-456',
                access_expires_at: nextExpiry,
                refresh_expires_at: nextExpiry,
            })

            // getRefreshToken stays null until after store creation/login so the
            // automatic init-time checkAuthStatus() does not itself trigger a refresh.
            const store = useAuthStore()
            await store.authReady
            await store.login({ username: 'alice', password: 'secret' })
            vi.mocked(getRefreshToken).mockReturnValue('refresh-123')

            expect(typedHttpClient.refreshToken).not.toHaveBeenCalled()

            // Fire the scheduled refresh timer (refreshTime = timeUntilExpiry - buffer).
            await vi.advanceTimersByTimeAsync(accessExpiresInMs)

            expect(typedHttpClient.refreshToken).toHaveBeenCalledWith({
                refresh_token: 'refresh-123',
            })
            expect(store.token).toBe(refreshedToken)
        })
    })

    describe('logout', () => {
        it('revokes the refresh token on the backend and clears local state', async () => {
            vi.mocked(getRefreshToken).mockReturnValue('refresh-abc')
            vi.mocked(typedHttpClient.logout).mockResolvedValue({ message: 'ok' })

            const store = useAuthStore()
            await store.authReady
            await store.logout()

            expect(typedHttpClient.logout).toHaveBeenCalledWith({ refresh_token: 'refresh-abc' })
            expect(store.token).toBeNull()
            expect(store.user).toBeNull()
            expect(store.isAuthenticated).toBe(false)
        })

        it('still clears local state when the backend logout call fails', async () => {
            vi.mocked(getRefreshToken).mockReturnValue('refresh-abc')
            vi.mocked(typedHttpClient.logout).mockRejectedValue(new Error('network down'))

            const store = useAuthStore()
            await store.authReady
            await expect(store.logout()).resolves.toBeUndefined()

            expect(store.token).toBeNull()
            expect(store.isAuthenticated).toBe(false)
        })

        it('skips the backend call entirely when there is no refresh token', async () => {
            vi.mocked(getRefreshToken).mockReturnValue(null)

            const store = useAuthStore()
            await store.authReady
            await store.logout()

            expect(typedHttpClient.logout).not.toHaveBeenCalled()
        })
    })

    describe('refreshTokens', () => {
        it('updates token, user and permissions on success', async () => {
            vi.mocked(getRefreshToken).mockReturnValue('refresh-abc')
            const newAccessToken = makeJwt({ sub: 'user-9', name: 'bob', permissions: ['x'] })
            vi.mocked(typedHttpClient.refreshToken).mockResolvedValue({
                access_token: newAccessToken,
                refresh_token: 'refresh-def',
                access_expires_at: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
                refresh_expires_at: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
            })
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: ['x'],
                allowed_routes: [],
            })

            const store = useAuthStore()
            await store.authReady
            await store.refreshTokens()

            expect(store.token).toBe(newAccessToken)
            expect(store.user?.uuid).toBe('user-9')
            expect(store.user?.username).toBe('bob')
            expect(setRefreshToken).toHaveBeenCalledWith('refresh-def', expect.any(Date))
        })

        it('logs out when there is no refresh token to use', async () => {
            vi.mocked(getRefreshToken).mockReturnValue(null)

            const store = useAuthStore()
            await store.authReady
            await store.refreshTokens()

            expect(typedHttpClient.refreshToken).not.toHaveBeenCalled()
            expect(store.token).toBeNull()
        })

        it('logs out when the backend refresh call fails', async () => {
            vi.mocked(getRefreshToken).mockReturnValue('refresh-abc')
            vi.mocked(typedHttpClient.refreshToken).mockRejectedValue(new Error('expired'))

            const store = useAuthStore()
            await store.authReady
            await store.refreshTokens()

            expect(store.token).toBeNull()
        })

        it('logs out when the refreshed access token cannot be parsed', async () => {
            vi.mocked(getRefreshToken).mockReturnValue('refresh-abc')
            vi.mocked(typedHttpClient.refreshToken).mockResolvedValue({
                access_token: 'not-a-valid-jwt',
                refresh_token: 'refresh-def',
                access_expires_at: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
                refresh_expires_at: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
            })

            const store = useAuthStore()
            await store.authReady
            await store.refreshTokens()

            expect(store.token).toBeNull()
        })

        it('shares a single in-flight refresh between concurrent callers', async () => {
            const newAccessToken = makeJwt({ sub: 'user-9', name: 'bob' })
            vi.mocked(typedHttpClient.refreshToken).mockResolvedValue({
                access_token: newAccessToken,
                refresh_token: 'refresh-def',
                access_expires_at: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
                refresh_expires_at: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
            })
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: [],
                allowed_routes: [],
            })

            // getRefreshToken stays null until after store creation so the automatic
            // init-time checkAuthStatus() does not itself consume the in-flight refresh.
            const store = useAuthStore()
            await store.authReady
            vi.mocked(getRefreshToken).mockReturnValue('refresh-abc')

            const [a, b] = await Promise.all([store.refreshTokens(), store.refreshTokens()])

            expect(a).toBeUndefined()
            expect(b).toBeUndefined()
            expect(typedHttpClient.refreshToken).toHaveBeenCalledTimes(1)
        })
    })

    describe('checkAuthStatus', () => {
        it('automatically refreshes when a refresh token exists but there is no access token', async () => {
            vi.mocked(getRefreshToken).mockReturnValue('refresh-abc')
            const newAccessToken = makeJwt({ sub: 'user-9', name: 'bob' })
            vi.mocked(typedHttpClient.refreshToken).mockResolvedValue({
                access_token: newAccessToken,
                refresh_token: 'refresh-def',
                access_expires_at: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
                refresh_expires_at: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
            })
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: [],
                allowed_routes: [],
            })

            const store = useAuthStore()
            await store.authReady

            expect(typedHttpClient.refreshToken).toHaveBeenCalledWith({
                refresh_token: 'refresh-abc',
            })
            expect(store.isAuthenticated).toBe(true)
        })

        it('refreshes an expired token when a refresh token is available', async () => {
            // Short-lived JWT (exp already passed) but far-future access_expires_at so
            // establishSession's own setupTokenRefresh call does not race with the
            // manual checkAuthStatus call below.
            const expiredJwt = makeJwt({ exp: Math.floor(Date.now() / 1000) - 60 })
            const futureExpiry = new Date(Date.now() + 60 * 60 * 1000).toISOString()

            vi.mocked(typedHttpClient.login).mockResolvedValue({
                access_token: expiredJwt,
                refresh_token: 'refresh-123',
                user_uuid: 'user-uuid-1',
                username: 'alice',
                access_expires_at: futureExpiry,
                refresh_expires_at: futureExpiry,
                using_default_password: false,
            })
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: [],
                allowed_routes: [],
            })

            const store = useAuthStore()
            await store.authReady
            await store.login({ username: 'alice', password: 'secret' })
            expect(store.isTokenExpired).toBe(true)

            const refreshedToken = makeJwt({ sub: 'user-uuid-1', name: 'alice' })
            vi.mocked(getRefreshToken).mockReturnValue('refresh-123')
            vi.mocked(typedHttpClient.refreshToken).mockResolvedValue({
                access_token: refreshedToken,
                refresh_token: 'refresh-456',
                access_expires_at: futureExpiry,
                refresh_expires_at: futureExpiry,
            })

            await store.checkAuthStatus()

            expect(typedHttpClient.refreshToken).toHaveBeenCalledWith({
                refresh_token: 'refresh-123',
            })
            expect(store.token).toBe(refreshedToken)
        })

        it('logs out an expired token when no refresh token is available', async () => {
            const expiredJwt = makeJwt({ exp: Math.floor(Date.now() / 1000) - 60 })
            const futureExpiry = new Date(Date.now() + 60 * 60 * 1000).toISOString()

            vi.mocked(typedHttpClient.login).mockResolvedValue({
                access_token: expiredJwt,
                refresh_token: 'refresh-123',
                user_uuid: 'user-uuid-1',
                username: 'alice',
                access_expires_at: futureExpiry,
                refresh_expires_at: futureExpiry,
                using_default_password: false,
            })
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: [],
                allowed_routes: [],
            })

            const store = useAuthStore()
            await store.authReady
            await store.login({ username: 'alice', password: 'secret' })
            expect(store.isTokenExpired).toBe(true)

            vi.mocked(getRefreshToken).mockReturnValue(null)
            await store.checkAuthStatus()

            expect(store.token).toBeNull()
        })
    })

    describe('adoptSsoSession', () => {
        it('establishes a session from SSO tokens, reading identity from the JWT claims', async () => {
            const futureExpiry = new Date(Date.now() + 60 * 60 * 1000).toISOString()
            const ssoToken = makeJwt({ sub: 'sso-user-1', name: 'ssouser', permissions: [] })
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: [],
                allowed_routes: [],
            })

            const store = useAuthStore()
            await store.authReady
            await store.adoptSsoSession({
                access_token: ssoToken,
                refresh_token: 'sso-refresh',
                access_expires_at: futureExpiry,
                refresh_expires_at: futureExpiry,
            })

            expect(store.token).toBe(ssoToken)
            expect(store.user?.uuid).toBe('sso-user-1')
            expect(store.user?.username).toBe('ssouser')
            expect(store.isDefaultPasswordInUse).toBe(false)
        })

        it('rejects and leaves the session unestablished when establishSession throws', async () => {
            vi.mocked(setRefreshToken).mockImplementationOnce(() => {
                throw new Error('cookie write failed')
            })
            const futureExpiry = new Date(Date.now() + 60 * 60 * 1000).toISOString()
            const ssoToken = makeJwt({ sub: 'sso-user-1', name: 'ssouser' })

            const store = useAuthStore()
            await store.authReady

            await expect(
                store.adoptSsoSession({
                    access_token: ssoToken,
                    refresh_token: 'sso-refresh',
                    access_expires_at: futureExpiry,
                    refresh_expires_at: futureExpiry,
                })
            ).rejects.toThrow()

            expect(store.isAuthenticated).toBe(false)
        })
    })

    describe('SSO fragment adoption (runs automatically on store creation)', () => {
        it('adopts a full token pair from the URL fragment and clears it', async () => {
            const futureUnix = Math.floor(Date.now() / 1000) + 3600
            const ssoToken = makeJwt({ sub: 'sso-user-2', name: 'fragment-user' })
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: [],
                allowed_routes: [],
            })
            window.location.hash =
                `access_token=${ssoToken}&refresh_token=rt-1` +
                `&access_expires_at=${futureUnix}&refresh_expires_at=${futureUnix}`

            const store = useAuthStore()
            await store.authReady

            expect(store.isAuthenticated).toBe(true)
            expect(store.user?.username).toBe('fragment-user')
            expect(window.location.hash).toBe('')
        })

        it('records sso_error from the fragment and clears it', async () => {
            window.location.hash = 'sso_error=access_denied'

            const store = useAuthStore()
            await store.authReady

            expect(store.ssoError).toBe('access_denied')
            expect(window.location.hash).toBe('')
        })

        it('leaves the fragment untouched when access/refresh tokens are missing', async () => {
            window.location.hash = 'return_to=%2Fdashboard'

            const store = useAuthStore()
            await store.authReady

            expect(store.ssoError).toBeNull()
            expect(store.isAuthenticated).toBe(false)
            expect(window.location.hash).toBe('#return_to=%2Fdashboard')
        })

        it('sets a generic sso error when adopting the fragment session throws', async () => {
            vi.mocked(setRefreshToken).mockImplementationOnce(() => {
                throw new Error('cookie write failed')
            })
            const futureUnix = Math.floor(Date.now() / 1000) + 3600
            const ssoToken = makeJwt({ sub: 'sso-user-3', name: 'broken-user' })
            window.location.hash =
                `access_token=${ssoToken}&refresh_token=rt-1` +
                `&access_expires_at=${futureUnix}&refresh_expires_at=${futureUnix}`

            const store = useAuthStore()
            await store.authReady

            expect(store.ssoError).toBe('generic')
            expect(store.isAuthenticated).toBe(false)
        })
    })

    describe('banner dismissal', () => {
        it('dismissDefaultPasswordBanner hides the banner and persists the choice', async () => {
            const futureExpiry = new Date(Date.now() + 60 * 60 * 1000).toISOString()
            vi.mocked(typedHttpClient.login).mockResolvedValue({
                access_token: makeJwt({ permissions: [] }),
                refresh_token: 'refresh-123',
                user_uuid: 'user-uuid-1',
                username: 'alice',
                access_expires_at: futureExpiry,
                refresh_expires_at: futureExpiry,
                using_default_password: true,
            })
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: false,
                permissions: [],
                allowed_routes: [],
            })

            const store = useAuthStore()
            await store.authReady
            await store.login({ username: 'alice', password: 'secret' })

            expect(store.isDefaultPasswordInUse).toBe(true)

            store.dismissDefaultPasswordBanner()

            expect(store.isDefaultPasswordInUse).toBe(false)
            expect(localStorage.getItem('default_password_banner_dismissed')).toBe('true')
        })

        it('dismissMobileWarningBanner flips isMobileWarningDismissed and persists it', async () => {
            const store = useAuthStore()
            await store.authReady

            expect(store.isMobileWarningDismissed).toBe(false)

            store.dismissMobileWarningBanner()

            expect(store.isMobileWarningDismissed).toBe(true)
            expect(localStorage.getItem('mobile_warning_banner_dismissed')).toBe('true')
        })
    })

    describe('clearAuthState', () => {
        it('clears token, user and permissions without calling the backend', async () => {
            const futureExpiry = new Date(Date.now() + 60 * 60 * 1000).toISOString()
            vi.mocked(typedHttpClient.login).mockResolvedValue({
                access_token: makeJwt({ permissions: ['x'], is_super_admin: true }),
                refresh_token: 'refresh-123',
                user_uuid: 'user-uuid-1',
                username: 'alice',
                access_expires_at: futureExpiry,
                refresh_expires_at: futureExpiry,
                using_default_password: false,
            })
            vi.mocked(typedHttpClient.getUserPermissions).mockResolvedValue({
                is_super_admin: true,
                permissions: ['x'],
                allowed_routes: ['/x'],
            })

            const store = useAuthStore()
            await store.authReady
            await store.login({ username: 'alice', password: 'secret' })
            expect(store.isAuthenticated).toBe(true)

            store.clearAuthState()

            expect(store.token).toBeNull()
            expect(store.user).toBeNull()
            expect(store.isSuperAdmin).toBe(false)
            expect(store.permissions).toEqual([])
            expect(typedHttpClient.logout).not.toHaveBeenCalled()
            expect(mockClearVersions).toHaveBeenCalled()
        })
    })

    describe('clearError', () => {
        it('resets the error message back to null', async () => {
            vi.mocked(typedHttpClient.login).mockRejectedValueOnce(
                new HttpError(403, 'auth', 'create', 'Account locked or not active')
            )

            const store = useAuthStore()
            await store.authReady
            await expect(store.login({ username: 'u', password: 'p' })).rejects.toThrow()
            expect(store.error).toBe('locked')

            store.clearError()

            expect(store.error).toBeNull()
        })
    })

    describe('isTokenExpired', () => {
        it('is true when there is no token at all', async () => {
            const store = useAuthStore()
            await store.authReady

            expect(store.isTokenExpired).toBe(true)
        })
    })
})
