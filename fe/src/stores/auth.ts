import { defineStore } from 'pinia'
import { ref, computed, readonly } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { env } from '@/env-check'
import type { LoginRequest, LoginResponse, User } from '@/types/schemas'

export const useAuthStore = defineStore('auth', () => {
    // State
    const token = ref<string | null>(localStorage.getItem('auth_token'))
    const user = ref<User | null>(null)
    const refreshTimer = ref<number | null>(null)
    const isLoading = ref(false)
    const error = ref<string | null>(null)

    // Getters
    const isAuthenticated = computed(() => !!token.value && !!user.value)
    const isTokenExpired = computed(() => {
        if (!token.value) return true
        
        try {
            const payload = JSON.parse(atob(token.value.split('.')[1]))
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
            
            // Store token and user info
            token.value = response.token
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

            // Store token in localStorage
            localStorage.setItem('auth_token', response.token)
            
            // Set up automatic token refresh
            setupTokenRefresh(response.expires_at)
            
            if (env.enableApiLogging) {
                console.log('[Auth] Login successful:', {
                    username: response.username,
                    role: response.role,
                    expires_at: response.expires_at
                })
            }
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Login failed'
            error.value = errorMessage
            
            if (env.enableApiLogging) {
                console.error('[Auth] Login failed:', errorMessage)
            }
            
            throw new Error(errorMessage)
        } finally {
            isLoading.value = false
        }
    }

    const logout = (): void => {
        // Clear state
        token.value = null
        user.value = null
        error.value = null
        
        // Clear localStorage
        localStorage.removeItem('auth_token')
        
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
                refreshToken()
            }, refreshTime)
            
            if (env.enableApiLogging) {
                console.log(`[Auth] Token refresh scheduled in ${Math.round(refreshTime / 1000)} seconds`)
            }
        } else {
            // Token is already expired or about to expire
            if (env.enableApiLogging) {
                console.warn('[Auth] Token is expired or about to expire, logging out')
            }
            logout()
        }
    }

    const refreshToken = async (): Promise<void> => {
        if (!token.value) {
            return
        }

        try {
            // TODO: Implement token refresh endpoint when available in backend
            // For now, we'll just check if the token is still valid
            // In a real implementation, you would call a refresh endpoint:
            // const response = await typedHttpClient.refreshToken()
            
            if (env.enableApiLogging) {
                console.log('[Auth] Token refresh not yet implemented in backend')
            }
        } catch (err) {
            if (env.enableApiLogging) {
                console.error('[Auth] Token refresh failed:', err)
            }
            logout()
        }
    }

    const checkAuthStatus = (): void => {
        if (token.value && !user.value) {
            // We have a token but no user data, try to restore from token
            try {
                const payload = JSON.parse(atob(token.value.split('.')[1]))
                
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
        } else if (token.value && isTokenExpired.value) {
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
        token: readonly(token),
        user: readonly(user),
        isLoading: readonly(isLoading),
        error: readonly(error),
        
        // Getters
        isAuthenticated,
        isTokenExpired,
        
        // Actions
        login,
        logout,
        refreshToken,
        checkAuthStatus,
        clearError,
    }
})
