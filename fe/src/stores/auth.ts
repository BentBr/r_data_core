import { defineStore } from 'pinia'
import { ref, computed, readonly } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { env } from '@/env-check'
import { useTranslations } from '@/composables/useTranslations'
import { getRefreshToken, setRefreshToken, deleteRefreshToken } from '@/utils/cookies'
import type { LoginRequest, User } from '@/types/schemas'

export const useAuthStore = defineStore('auth', () => {
    // Translation system
    const { translateError } = useTranslations()

    // State - access token only in memory, refresh token in secure cookie
    const access_token = ref<string | null>(null)
    const user = ref<User | null>(null)
    const refreshTimer = ref<ReturnType<typeof setTimeout> | null>(null)
    const isLoading = ref(false)
    const error = ref<string | null>(null)
    const isRefreshing = ref(false) // Flag to prevent concurrent refresh attempts
    const permissions = ref<string[]>([])
    const isSuperAdmin = ref(false)
    const allowedRoutes = ref<string[]>([])
    const usingDefaultPassword = ref(false)
    const defaultPasswordBannerDismissed = ref(false)
    const mobileWarningDismissed = ref(false)

    // LocalStorage keys for banner dismissal
    const DISMISSED_BANNER_KEY = 'default_password_banner_dismissed'
    const MOBILE_WARNING_DISMISSED_KEY = 'mobile_warning_banner_dismissed'

    // Initialize dismissed states from localStorage
    if (typeof window !== 'undefined') {
        const defaultPasswordDismissed = localStorage.getItem(DISMISSED_BANNER_KEY)
        defaultPasswordBannerDismissed.value = defaultPasswordDismissed === 'true'

        const mobileDismissed = localStorage.getItem(MOBILE_WARNING_DISMISSED_KEY)
        mobileWarningDismissed.value = mobileDismissed === 'true'
    }

    // Getters
    const isAuthenticated = computed(() => !!access_token.value && !!user.value)
    const isDefaultPasswordInUse = computed(() => {
        // Return false if password changed (false)
        if (!usingDefaultPassword.value) {
            return false
        }
        // Check if banner was dismissed (using reactive ref)
        if (defaultPasswordBannerDismissed.value) {
            return false
        }
        // Only return true if using default password and not dismissed
        return usingDefaultPassword.value
    })
    const isTokenExpired = computed(() => {
        if (!access_token.value) {
            return true
        }

        try {
            const payload = JSON.parse(atob(access_token.value.split('.')[1]))
            const exp = payload.exp * 1000 // Convert to milliseconds
            const now = Date.now()
            const bufferTime = env.tokenRefreshBuffer * 60 * 1000 // Convert minutes to milliseconds

            return exp - bufferTime <= now
        } catch {
            return true
        }
    })

    // Actions
    const login = async (credentials: LoginRequest): Promise<void> => {
        isLoading.value = true
        error.value = null

        try {
            const response = await typedHttpClient.login(credentials)

            // Store tokens and user info
            access_token.value = response.access_token

            // Store refresh token in secure cookie
            const refreshExpiresAt = new Date(
                response.refresh_expires_at || Date.now() + 30 * 24 * 60 * 60 * 1000
            )
            setRefreshToken(response.refresh_token, refreshExpiresAt)

            user.value = {
                uuid: response.user_uuid,
                username: response.username,
                role_uuids: [], // Will be loaded separately if needed
                // Set default values for required User fields not in LoginResponse
                email: '',
                first_name: '',
                last_name: '',
                is_active: true,
                is_admin: false, // Will be determined from permissions
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            }

            // Decode and store permissions from JWT
            decodeAndStorePermissions(response.access_token)

            // Load permissions and allowed routes from API
            await loadUserPermissions()

            // Set up automatic token refresh
            setupTokenRefresh(response.access_expires_at)

            // Store default password check result
            usingDefaultPassword.value = response.using_default_password

            if (env.enableApiLogging) {
                console.log('[Auth] Login successful:', {
                    username: response.username,
                    expires_at: response.access_expires_at,
                    using_default_password: usingDefaultPassword.value,
                })
            }
        } catch (err) {
            const rawErrorMessage = err instanceof Error ? err.message : 'Login failed'
            const translatedErrorMessage = translateError(rawErrorMessage)
            error.value = translatedErrorMessage

            if (env.enableApiLogging) {
                console.error('[Auth] Login failed:', {
                    rawError: rawErrorMessage,
                    translatedError: translatedErrorMessage,
                })
            }

            throw new Error(translatedErrorMessage)
        } finally {
            isLoading.value = false
        }
    }

    const dismissDefaultPasswordBanner = (): void => {
        defaultPasswordBannerDismissed.value = true
        if (typeof window !== 'undefined') {
            localStorage.setItem(DISMISSED_BANNER_KEY, 'true')
        }
        if (env.enableApiLogging) {
            console.log('[Auth] Default password banner dismissed')
        }
    }

    // Mobile warning banner
    const isMobileWarningDismissed = computed(() => mobileWarningDismissed.value)

    const dismissMobileWarningBanner = (): void => {
        mobileWarningDismissed.value = true
        if (typeof window !== 'undefined') {
            localStorage.setItem(MOBILE_WARNING_DISMISSED_KEY, 'true')
        }
        if (env.enableApiLogging) {
            console.log('[Auth] Mobile warning banner dismissed')
        }
    }

    const clearAuthState = (): void => {
        // Clear state immediately to prevent API calls
        access_token.value = null
        user.value = null
        permissions.value = []
        isSuperAdmin.value = false
        allowedRoutes.value = []
        error.value = null
        usingDefaultPassword.value = false

        // Reset dismissed banner states so they show again for next login
        defaultPasswordBannerDismissed.value = false
        mobileWarningDismissed.value = false
        if (typeof window !== 'undefined') {
            localStorage.removeItem(DISMISSED_BANNER_KEY)
            localStorage.removeItem(MOBILE_WARNING_DISMISSED_KEY)
        }

        // Clear refresh token from secure cookie
        deleteRefreshToken()

        // Clear refresh timer
        if (refreshTimer.value) {
            clearTimeout(refreshTimer.value)
            refreshTimer.value = null
        }

        if (env.enableApiLogging) {
            console.log('[Auth] Auth state cleared immediately')
        }
    }

    const logout = async (): Promise<void> => {
        const refreshToken = getRefreshToken()

        // First, try to revoke the refresh token on the backend via logout endpoint
        if (refreshToken) {
            try {
                if (env.enableApiLogging) {
                    console.log('[Auth] Logging out and revoking refresh token on backend...')
                }

                // Call the logout endpoint which will revoke the refresh token
                await typedHttpClient.logout({ refresh_token: refreshToken })

                if (env.enableApiLogging) {
                    console.log('[Auth] Logout successful, refresh token revoked')
                }
            } catch (err) {
                // Don't fail logout if backend call fails, just log it
                if (env.enableApiLogging) {
                    console.error('[Auth] Failed to logout on backend:', err)
                }
            }
        }

        // Clear state
        access_token.value = null
        user.value = null
        permissions.value = []
        isSuperAdmin.value = false
        allowedRoutes.value = []
        error.value = null
        usingDefaultPassword.value = false

        // Reset dismissed banner states so they show again for next login
        defaultPasswordBannerDismissed.value = false
        mobileWarningDismissed.value = false
        if (typeof window !== 'undefined') {
            localStorage.removeItem(DISMISSED_BANNER_KEY)
            localStorage.removeItem(MOBILE_WARNING_DISMISSED_KEY)
        }

        // Clear refresh token from secure cookie
        deleteRefreshToken()

        // Clear refresh timer
        if (refreshTimer.value) {
            clearTimeout(refreshTimer.value)
            refreshTimer.value = null
        }

        if (env.enableApiLogging) {
            console.log('[Auth] Logout completed')
        }
    }

    const setupTokenRefresh = (expiresAt: string): void => {
        if (refreshTimer.value) {
            clearTimeout(refreshTimer.value)
        }

        const expiresAtDate = new Date(expiresAt)
        const now = new Date()
        const timeUntilExpiry = expiresAtDate.getTime() - now.getTime()
        const bufferTime = env.tokenRefreshBuffer * 60 * 1000 // Convert minutes to milliseconds

        if (timeUntilExpiry > bufferTime) {
            // Set up refresh timer
            const refreshTime = timeUntilExpiry - bufferTime
            refreshTimer.value = setTimeout(() => {
                if (env.enableApiLogging) {
                    console.log('[Auth] Token refresh timer triggered')
                }
                void refreshTokens()
            }, refreshTime)

            if (env.enableApiLogging) {
                console.log(
                    '[Auth] Token refresh scheduled for:',
                    new Date(Date.now() + refreshTime)
                )
            }
        } else {
            // Token is already expired or about to expire
            if (env.enableApiLogging) {
                console.warn('[Auth] Token is expired or about to expire, logging out')
            }
            logout().catch(err => {
                if (env.enableApiLogging) {
                    console.error('[Auth] Logout failed during token refresh setup:', err)
                }
            })
        }
    }

    const refreshTokens = async (): Promise<void> => {
        // Prevent concurrent refresh attempts
        if (isRefreshing.value) {
            if (env.enableApiLogging) {
                console.log('[Auth] Refresh already in progress, skipping')
            }
            return
        }

        const refreshToken = getRefreshToken()
        if (!refreshToken) {
            if (env.enableApiLogging) {
                console.warn('[Auth] No refresh token available')
            }
            await logout()
            return
        }

        isRefreshing.value = true

        try {
            if (env.enableApiLogging) {
                console.log('[Auth] Refreshing access token...')
            }

            const response = await typedHttpClient.refreshToken({
                refresh_token: refreshToken,
            })

            // Update tokens
            access_token.value = response.access_token

            // Update refresh token in secure cookie
            const refreshExpiresAt = new Date(
                response.refresh_expires_at || Date.now() + 30 * 24 * 60 * 60 * 1000
            )
            setRefreshToken(response.refresh_token, refreshExpiresAt)

            // Restore user data from the new access token
            try {
                const payload = JSON.parse(atob(response.access_token.split('.')[1]))
                user.value = {
                    uuid: payload.sub ?? '',
                    username: payload.name ?? payload.username ?? '',
                    role_uuids: [], // Roles are not stored in JWT
                    email: payload.email ?? '',
                    first_name: '',
                    last_name: '',
                    is_active: true,
                    is_admin: false,
                    created_at: new Date().toISOString(),
                    updated_at: new Date().toISOString(),
                }
            } catch (err) {
                if (env.enableApiLogging) {
                    console.error('[Auth] Failed to parse user data from token:', err)
                }
                // If we can't parse the token, we should logout
                await logout()
                return
            }

            // Decode and store permissions from new token
            decodeAndStorePermissions(response.access_token)

            // Load permissions and allowed routes from API
            await loadUserPermissions()

            // Set up next refresh
            setupTokenRefresh(response.access_expires_at)

            if (env.enableApiLogging) {
                console.log('[Auth] Token refresh successful')
            }
        } catch (err) {
            if (env.enableApiLogging) {
                console.error('[Auth] Token refresh failed:', err)
            }
            await logout()
        } finally {
            isRefreshing.value = false
        }
    }

    const checkAuthStatus = async (): Promise<void> => {
        // Check if we have a refresh token but no access token
        const refreshToken = getRefreshToken()

        if (env.enableApiLogging) {
            console.log('[Auth] checkAuthStatus called:', {
                hasAccessToken: !!access_token.value,
                hasRefreshToken: !!refreshToken,
                hasUser: !!user.value,
                isAuthenticated: isAuthenticated.value,
                isTokenExpired: isTokenExpired.value,
            })
        }

        if (!access_token.value && refreshToken) {
            // Try to refresh the token automatically
            if (env.enableApiLogging) {
                console.log(
                    '[Auth] No access token but refresh token exists, attempting automatic refresh'
                )
            }
            try {
                await refreshTokens()
                if (env.enableApiLogging) {
                    console.log('[Auth] Automatic refresh successful')
                }
                return
            } catch (err) {
                if (env.enableApiLogging) {
                    console.error('[Auth] Automatic refresh failed:', err)
                }
                // Don't logout here, let the user try to login manually
                return
            }
        }

        if (access_token.value && !user.value) {
            // We have a token but no user data, try to restore from token
            try {
                const payload = JSON.parse(atob(access_token.value.split('.')[1]))

                if (payload.exp * 1000 > Date.now()) {
                    // Token is still valid, restore basic user info
                    user.value = {
                        uuid: payload.sub ?? '',
                        username: payload.name ?? payload.username ?? '',
                        role_uuids: [], // Roles are not stored in JWT
                        email: payload.email ?? '',
                        first_name: '',
                        last_name: '',
                        is_active: true,
                        is_admin: false,
                        created_at: new Date().toISOString(),
                        updated_at: new Date().toISOString(),
                    }

                    // Set up token refresh
                    setupTokenRefresh(new Date(payload.exp * 1000).toISOString())

                    if (env.enableApiLogging) {
                        console.log('[Auth] User restored from token')
                    }
                } else {
                    // Token is expired, try to refresh
                    if (refreshToken) {
                        try {
                            await refreshTokens()
                        } catch (err) {
                            if (env.enableApiLogging) {
                                console.error('[Auth] Token refresh failed:', err)
                            }
                            await logout()
                        }
                    } else {
                        await logout()
                    }
                }
            } catch {
                // Invalid token, try to refresh if we have refresh token
                if (refreshToken) {
                    try {
                        await refreshTokens()
                    } catch (err) {
                        if (env.enableApiLogging) {
                            console.error('[Auth] Token refresh failed:', err)
                        }
                        await logout()
                    }
                } else {
                    await logout()
                }
            }
        } else if (access_token.value && isTokenExpired.value) {
            // Token is expired, try to refresh
            if (refreshToken) {
                try {
                    await refreshTokens()
                } catch (err) {
                    if (env.enableApiLogging) {
                        console.error('[Auth] Token refresh failed:', err)
                    }
                    await logout()
                }
            } else {
                await logout()
            }
        }
    }

    const clearError = (): void => {
        error.value = null
    }

    const decodeAndStorePermissions = (token: string): void => {
        try {
            const payload = JSON.parse(atob(token.split('.')[1]))
            permissions.value = (payload.permissions as string[]) || []
            isSuperAdmin.value = payload.is_super_admin === true
        } catch (err) {
            if (env.enableApiLogging) {
                console.error('[Auth] Failed to decode permissions from token:', err)
            }
            permissions.value = []
            isSuperAdmin.value = false
        }
    }

    const loadUserPermissions = async (): Promise<void> => {
        try {
            const response = await typedHttpClient.getUserPermissions()
            isSuperAdmin.value = response.is_super_admin || false
            allowedRoutes.value = response.allowed_routes || []
            permissions.value = response.permissions || []
        } catch (err) {
            if (env.enableApiLogging) {
                console.error('[Auth] Failed to load user permissions:', err)
            }
            // Don't throw, just use empty permissions
            allowedRoutes.value = []
            permissions.value = []
        }
    }

    const canAccessRoute = (route: string): boolean => {
        // Super admin can access all routes
        if (isSuperAdmin.value) {
            return true
        }
        // Check if route is in allowed routes
        return allowedRoutes.value.includes(route)
    }

    // Helper function to convert frontend namespace format to backend format
    // Frontend: "Workflows", "EntityDefinitions", "ApiKeys"
    // Backend: "workflows", "entity_definitions", "api_keys"
    const convertNamespaceToBackendFormat = (namespace: string): string => {
        // Convert PascalCase to snake_case and lowercase
        // EntityDefinitions -> entity_definitions
        // ApiKeys -> api_keys
        // Workflows -> workflows
        return namespace
            .replace(/([A-Z])/g, '_$1')
            .toLowerCase()
            .replace(/^_/, '') // Remove leading underscore
    }

    const hasPermission = (namespace: string, permissionType: string): boolean => {
        // Global admin: Super admin has all permissions for all namespaces
        if (isSuperAdmin.value) {
            return true
        }
        // Convert namespace to backend format (workflows, entity_definitions, api_keys, etc.)
        const namespaceBackend = convertNamespaceToBackendFormat(namespace)
        // Resource-level admin: Check if user has Admin permission for this namespace
        // Admin permission grants all permission types for the namespace
        const adminPermissionString = `${namespaceBackend}:admin`
        if (permissions.value.includes(adminPermissionString)) {
            return true
        }
        // Exact permission check: Check if user has the specific permission
        const permissionString = `${namespaceBackend}:${permissionType.toLowerCase()}`
        return permissions.value.includes(permissionString)
    }

    // Initialize auth status on store creation
    // Note: We can't await in the store initialization, so we'll call it without await
    // The router guard will handle the async auth check
    checkAuthStatus().catch(err => {
        if (env.enableApiLogging) {
            console.error('[Auth] Initial auth check failed:', err)
        }
    })

    return {
        // State
        token: readonly(access_token),
        user: readonly(user),
        isLoading: readonly(isLoading),
        error: readonly(error),
        permissions: readonly(permissions),
        isSuperAdmin: readonly(isSuperAdmin),
        allowedRoutes: readonly(allowedRoutes),

        // Getters
        isAuthenticated,
        isTokenExpired,
        isDefaultPasswordInUse,
        isMobileWarningDismissed,

        // Actions
        login,
        logout,
        refreshTokens,
        checkAuthStatus,
        clearError,
        clearAuthState,
        canAccessRoute,
        hasPermission,
        loadUserPermissions,
        dismissDefaultPasswordBanner,
        dismissMobileWarningBanner,
    }
})
