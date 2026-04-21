import { defineStore } from 'pinia'
import { ref, readonly } from 'vue'
import { typedHttpClient } from '@/api/typed-client'

export const useCapabilitiesStore = defineStore('capabilities', () => {
    const systemMailConfigured = ref(false)
    const workflowMailConfigured = ref(false)
    const isLoaded = ref(false)

    async function fetchCapabilities() {
        try {
            const response = await typedHttpClient.getCapabilities()
            systemMailConfigured.value = response.system_mail_configured
            workflowMailConfigured.value = response.workflow_mail_configured
            isLoaded.value = true
        } catch (error) {
            console.error('Failed to fetch capabilities:', error)
            // Defaults to false — features hidden when API fails
            isLoaded.value = true
        }
    }

    return {
        systemMailConfigured: readonly(systemMailConfigured),
        workflowMailConfigured: readonly(workflowMailConfigured),
        isLoaded: readonly(isLoaded),
        fetchCapabilities,
    }
})
