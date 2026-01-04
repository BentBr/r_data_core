import { defineStore } from 'pinia'
import { ref, computed, readonly } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { env } from '@/env-check'
import type { SystemVersions, ComponentVersion } from '@/api/clients/system'

// Frontend version from package.json (injected at build time via Vite)
const FE_VERSION = __APP_VERSION__

export const useVersionStore = defineStore('versions', () => {
    // State
    const coreVersion = ref<string | null>(null)
    const workerVersion = ref<ComponentVersion | null>(null)
    const maintenanceVersion = ref<ComponentVersion | null>(null)
    const isLoading = ref(false)
    const error = ref<string | null>(null)
    const lastFetchedAt = ref<Date | null>(null)

    // Getters
    const feVersion = computed(() => FE_VERSION)

    const allVersions = computed(() => ({
        fe: FE_VERSION,
        core: coreVersion.value,
        worker: workerVersion.value?.version ?? null,
        maintenance: maintenanceVersion.value?.version ?? null,
    }))

    const hasVersions = computed(() => coreVersion.value !== null)

    // Actions
    const loadVersions = async (): Promise<void> => {
        // Skip if already loading
        if (isLoading.value) {
            return
        }

        isLoading.value = true
        error.value = null

        try {
            const response: SystemVersions = await typedHttpClient.getSystemVersions()

            coreVersion.value = response.core
            workerVersion.value = response.worker ?? null
            maintenanceVersion.value = response.maintenance ?? null
            lastFetchedAt.value = new Date()

            if (env.enableApiLogging) {
                console.log('[Versions] Loaded system versions:', {
                    fe: FE_VERSION,
                    core: response.core,
                    worker: response.worker?.version,
                    maintenance: response.maintenance?.version,
                })
            }
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : 'Failed to load versions'
            error.value = errorMessage

            if (env.enableApiLogging) {
                console.error('[Versions] Failed to load versions:', errorMessage)
            }
        } finally {
            isLoading.value = false
        }
    }

    const clearVersions = (): void => {
        coreVersion.value = null
        workerVersion.value = null
        maintenanceVersion.value = null
        lastFetchedAt.value = null
        error.value = null
    }

    return {
        // State
        coreVersion: readonly(coreVersion),
        workerVersion: readonly(workerVersion),
        maintenanceVersion: readonly(maintenanceVersion),
        isLoading: readonly(isLoading),
        error: readonly(error),
        lastFetchedAt: readonly(lastFetchedAt),

        // Getters
        feVersion,
        allVersions,
        hasVersions,

        // Actions
        loadVersions,
        clearVersions,
    }
})
