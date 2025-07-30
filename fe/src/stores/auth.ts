import { defineStore } from 'pinia'
import { ref, computed, readonly } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { env } from '@/env-check'
import { useTranslations } from '@/composables/useTranslations'
import type { LoginRequest, User } from '@/types/schemas'

export const useAuthStore = defineStore('auth', () => {
    // Translation system
    const { translateError } = useTranslations()

    // State
    const access_token = ref<string | null>(localStorage.getItem('auth_token'))
    const refreshToken = ref<string | null>(localStorage.getItem('refresh_token'))
    const user = ref<User | null>(null)
    const refreshTimer = ref<number | null>(null)
    const isLoading = ref(false)
    const error = ref<string | null>(null)

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
            refreshToken.value = response.refresh_token
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

            // Store tokens in localStorage
            localStorage.setItem('auth_token', response.access_token)
            localStorage.setItem('refresh_token', response.refresh_token)

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

    const logout = (): void => {
        // Clear state
        access_token.value = null
        refreshToken.value = null
        user.value = null
        error.value = null

        // Clear localStorage
        localStorage.removeItem('auth_token')
        localStorage.removeItem('refresh_token')

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

        const expirationTime = new Date(expiresAt).getTime()
        const now = Date.now()
        const bufferTime = env.tokenRefreshBuffer * 60 * 1000 // Convert minutes to milliseconds
        const refreshTime = expirationTime - now - bufferTime

        if (refreshTime > 0) {
            refreshTimer.value = window.setTimeout(() => {
                refreshTokens()
            }, refreshTime)

            if (env.enableApiLogging) {
                console.log(
                    `[Auth] Token refresh scheduled in ${Math.round(refreshTime / 1000)} seconds`
                )
            }
        } else {
            // Token is already expired or about to expire
            if (env.enableApiLogging) {
                console.warn('[Auth] Token is expired or about to expire, logging out')
            }
            logout()
        }
    }

    const refreshTokens = async (): Promise<void> => {
        if (!refreshToken.value) {
            if (env.enableApiLogging) {
                console.warn('[Auth] No refresh token available')
            }
            logout()
            return
        }

        try {
            if (env.enableApiLogging) {
                console.log('[Auth] Refreshing access token...')
            }

            const response = await typedHttpClient.refreshToken({
                refresh_token: refreshToken.value,
            })

            // Update tokens
            access_token.value = response.access_token
            refreshToken.value = response.refresh_token

            // Store new tokens in localStorage
            localStorage.setItem('auth_token', response.access_token)
            localStorage.setItem('refresh_token', response.refresh_token)

            // Set up next refresh
            setupTokenRefresh(response.access_expires_at)

            if (env.enableApiLogging) {
                console.log('[Auth] Token refresh successful')
            }
        } catch (err) {
            if (env.enableApiLogging) {
                console.error('[Auth] Token refresh failed:', err)
            }
            logout()
        }
    }

    const checkAuthStatus = (): void => {
        if (access_token.value && !user.value) {
            // We have a token but no user data, try to restore from token
            try {
                const payload = JSON.parse(atob(access_token.value.split('.')[1]))

                if (payload.exp * 1000 > Date.now()) {
                    // Token is still valid, restore basic user info
                    user.value = {
                        uuid: payload.sub || '',
                        username: payload.username || '',
                        role: payload.role || '',
                        email: '',
                        first_name: '',
                        last_name: '',
                        is_active: true,
                        created_at: new Date().toISOString(),
                        updated_at: new Date().toISOString(),
                    }

                    // Set up token refresh
                    setupTokenRefresh(new Date(payload.exp * 1000).toISOString())
                } else {
                    // Token is expired
                    logout()
                }
            } catch {
                // Invalid token
                logout()
            }
        } else if (access_token.value && isTokenExpired.value) {
            // Token is expired, logout
            logout()
        }
    }

    const clearError = (): void => {
        error.value = null
    }

    // Initialize auth status on store creation
    checkAuthStatus()

    return {
        // State
        token: readonly(access_token),
        user: readonly(user),
        isLoading: readonly(isLoading),
        error: readonly(error),

        // Getters
        isAuthenticated,
        isTokenExpired,

        // Actions
        login,
        logout,
        refreshTokens,
        checkAuthStatus,
        clearError,
    }
})
