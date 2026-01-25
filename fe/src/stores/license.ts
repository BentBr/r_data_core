import { defineStore } from 'pinia'
import { ref, computed, readonly } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { env } from '@/env-check'
import type { LicenseStatus } from '@/api/clients/system'

export const useLicenseStore = defineStore('license', () => {
    // State
    const licenseStatus = ref<LicenseStatus | null>(null)
    const isLoading = ref(false)
    const error = ref<string | null>(null)
    const licenseBannerDismissed = ref(false)

    // LocalStorage key for banner dismissal
    const DISMISSED_BANNER_KEY = 'license_banner_dismissed'

    // Initialize dismissed state from localStorage
    if (typeof window !== 'undefined') {
        const dismissed = localStorage.getItem(DISMISSED_BANNER_KEY)
        licenseBannerDismissed.value = dismissed === 'true'
    }

    // Getters
    const shouldShowBanner = computed(() => {
        if (!licenseStatus.value) {
            return false
        }

        // Don't show if dismissed
        if (licenseBannerDismissed.value) {
            return false
        }

        // Show if state is none, invalid, or error
        const state = licenseStatus.value.state
        return state === 'none' || state === 'invalid' || state === 'error'
    })

    const isLicenseValid = computed(() => {
        return licenseStatus.value?.state === 'valid'
    })

    // Actions
    const loadLicenseStatus = async (): Promise<void> => {
        isLoading.value = true
        error.value = null

        try {
            const status = await typedHttpClient.getLicenseStatus()
            licenseStatus.value = status

            if (env.enableApiLogging) {
                console.log('[License] License status loaded:', status)
            }
        } catch (err) {
            const errorMessage =
                err instanceof Error ? err.message : 'Failed to load license status'
            error.value = errorMessage

            if (env.enableApiLogging) {
                console.error('[License] Failed to load license status:', errorMessage)
            }
        } finally {
            isLoading.value = false
        }
    }

    const dismissLicenseBanner = (): void => {
        licenseBannerDismissed.value = true
        if (typeof window !== 'undefined') {
            localStorage.setItem(DISMISSED_BANNER_KEY, 'true')
        }
        if (env.enableApiLogging) {
            console.log('[License] License banner dismissed')
        }
    }

    const resetBannerDismissal = (): void => {
        licenseBannerDismissed.value = false
        if (typeof window !== 'undefined') {
            localStorage.removeItem(DISMISSED_BANNER_KEY)
        }
    }

    return {
        // State
        licenseStatus: readonly(licenseStatus),
        isLoading: readonly(isLoading),
        error: readonly(error),

        // Getters
        shouldShowBanner,
        isLicenseValid,

        // Actions
        loadLicenseStatus,
        dismissLicenseBanner,
        resetBannerDismissal,
    }
})
