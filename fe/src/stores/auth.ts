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

    // Getters
    const isAuthenticated = computed(() => !!access_token.value && !!user.value)
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
                role: response.role,
                // Set default values for required User fields not in LoginResponse
                email: '',
                first_name: '',
                last_name: '',
                is_active: true,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            }

            // Decode and store permissions from JWT
            decodeAndStorePermissions(response.access_token)

            // Load permissions and allowed routes from API
            await loadUserPermissions()

            // Set up automatic token refresh
            setupTokenRefresh(response.access_expires_at)

            if (env.enableApiLogging) {
                console.log('[Auth] Login successful:', {
                    username: response.username,
                    role: response.role,
                    expires_at: response.access_expires_at,
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

    const clearAuthState = (): void => {
        // Clear state immediately to prevent API calls
        access_token.value = null
        user.value = null
        permissions.value = []
        isSuperAdmin.value = false
        allowedRoutes.value = []
        error.value = null

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
        error.value = null

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
                    role: payload.role ?? '',
                    email: '',
                    first_name: '',
                    last_name: '',
                    is_active: true,
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
                        username: payload.username ?? '',
                        role: payload.role ?? '',
                        email: '',
                        first_name: '',
                        last_name: '',
                        is_active: true,
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

    const hasPermission = (namespace: string, permissionType: string): boolean => {
        // Super admin has all permissions
        if (isSuperAdmin.value) {
            return true
        }
        // Check if user has the specific permission
        const permissionString = `${namespace}:${permissionType.toLowerCase()}`
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
    }
})
