import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useCapabilitiesStore } from './capabilities'

// Mock the API client
vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getCapabilities: vi.fn(),
    },
}))

describe('useCapabilitiesStore', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.clearAllMocks()
    })

    it('defaults to false before fetch', () => {
        const store = useCapabilitiesStore()
        expect(store.systemMailConfigured).toBe(false)
        expect(store.workflowMailConfigured).toBe(false)
        expect(store.isLoaded).toBe(false)
    })

    it('fetches and exposes capabilities', async () => {
        const { typedHttpClient } = await import('@/api/typed-client')
        vi.mocked(typedHttpClient.getCapabilities).mockResolvedValueOnce({
            system_mail_configured: true,
            workflow_mail_configured: false,
            oidc_enabled: false,
            oidc_provider_name: null,
        })

        const store = useCapabilitiesStore()
        await store.fetchCapabilities()

        expect(store.systemMailConfigured).toBe(true)
        expect(store.workflowMailConfigured).toBe(false)
        expect(store.isLoaded).toBe(true)
    })

    it('reflects workflow_mail_configured when true', async () => {
        const { typedHttpClient } = await import('@/api/typed-client')
        vi.mocked(typedHttpClient.getCapabilities).mockResolvedValueOnce({
            system_mail_configured: false,
            workflow_mail_configured: true,
            oidc_enabled: false,
            oidc_provider_name: null,
        })

        const store = useCapabilitiesStore()
        await store.fetchCapabilities()

        expect(store.systemMailConfigured).toBe(false)
        expect(store.workflowMailConfigured).toBe(true)
        expect(store.isLoaded).toBe(true)
    })

    it('reflects both capabilities as true', async () => {
        const { typedHttpClient } = await import('@/api/typed-client')
        vi.mocked(typedHttpClient.getCapabilities).mockResolvedValueOnce({
            system_mail_configured: true,
            workflow_mail_configured: true,
            oidc_enabled: false,
            oidc_provider_name: null,
        })

        const store = useCapabilitiesStore()
        await store.fetchCapabilities()

        expect(store.systemMailConfigured).toBe(true)
        expect(store.workflowMailConfigured).toBe(true)
        expect(store.isLoaded).toBe(true)
    })

    it('handles API failure gracefully', async () => {
        const { typedHttpClient } = await import('@/api/typed-client')
        vi.mocked(typedHttpClient.getCapabilities).mockRejectedValueOnce(new Error('Network error'))

        const store = useCapabilitiesStore()
        await store.fetchCapabilities()

        expect(store.systemMailConfigured).toBe(false)
        expect(store.workflowMailConfigured).toBe(false)
        // isLoaded is true even on failure — features hidden, but fetch has completed
        expect(store.isLoaded).toBe(true)
    })

    it('sets isLoaded to true after successful fetch', async () => {
        const { typedHttpClient } = await import('@/api/typed-client')
        vi.mocked(typedHttpClient.getCapabilities).mockResolvedValueOnce({
            system_mail_configured: false,
            workflow_mail_configured: false,
            oidc_enabled: false,
            oidc_provider_name: null,
        })

        const store = useCapabilitiesStore()
        expect(store.isLoaded).toBe(false)

        await store.fetchCapabilities()

        expect(store.isLoaded).toBe(true)
    })

    it('exposes single sign-on when the server reports it enabled', async () => {
        const { typedHttpClient } = await import('@/api/typed-client')
        vi.mocked(typedHttpClient.getCapabilities).mockResolvedValueOnce({
            system_mail_configured: false,
            workflow_mail_configured: false,
            oidc_enabled: true,
            oidc_provider_name: 'Acme SSO',
        })

        const store = useCapabilitiesStore()
        await store.fetchCapabilities()

        expect(store.oidcEnabled).toBe(true)
        expect(store.oidcProviderName).toBe('Acme SSO')
    })

    it('keeps single sign-on off when the capabilities call fails', async () => {
        const { typedHttpClient } = await import('@/api/typed-client')
        vi.mocked(typedHttpClient.getCapabilities).mockRejectedValueOnce(new Error('offline'))

        const store = useCapabilitiesStore()
        await store.fetchCapabilities()

        // Failing closed matters here: a button that appears when the server
        // cannot confirm the feature exists leads straight to a dead end.
        expect(store.oidcEnabled).toBe(false)
        expect(store.isLoaded).toBe(true)
    })

    it('reports single sign-on without a provider name', async () => {
        const { typedHttpClient } = await import('@/api/typed-client')
        vi.mocked(typedHttpClient.getCapabilities).mockResolvedValueOnce({
            system_mail_configured: false,
            workflow_mail_configured: false,
            oidc_enabled: true,
            oidc_provider_name: null,
        })

        const store = useCapabilitiesStore()
        await store.fetchCapabilities()

        expect(store.oidcEnabled).toBe(true)
        expect(store.oidcProviderName).toBeNull()
    })
})
