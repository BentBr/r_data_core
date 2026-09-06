import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import LoginPage from './LoginPage.vue'

// ── per-file translation mock (overrides global test-setup) ─────────────────
vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (key: string) => {
            const map: Record<string, string> = {
                'auth.sso.sign_in': 'Sign in with single sign-on',
                'auth.sso.or': 'or',
                'auth.sso.errors.generic': 'Single sign-on failed.',
                'auth.sso.errors.not_permitted': 'Not permitted.',
            }
            return map[key] ?? key
        },
        translateError: (msg: string) => msg,
    }),
}))

vi.mock('@/components/common/LanguageSwitch.vue', () => ({
    default: { template: '<div />' },
}))
vi.mock('@/components/common/SmartIcon.vue', () => ({
    default: { template: '<span />', props: ['icon', 'size'] },
}))

// The capability values each test needs, read through a mutable object so a
// single module mock can serve every case.
const capabilities = {
    isLoaded: true,
    systemMailConfigured: false,
    oidcEnabled: false,
    fetchCapabilities: vi.fn(),
}

vi.mock('@/stores/capabilities', () => ({
    useCapabilitiesStore: () => capabilities,
}))

vi.mock('@/stores/auth')
vi.mock('@/api/typed-client', () => ({
    typedHttpClient: { forgotPassword: vi.fn() },
}))

import { useAuthStore } from '@/stores/auth'

function buildRouter() {
    const router = createRouter({
        history: createWebHistory(),
        routes: [
            { path: '/login', component: LoginPage },
            { path: '/dashboard', component: { template: '<div>Dashboard</div>' } },
        ],
    })
    void router.push('/login')
    return router
}

function buildAuthStoreMock(overrides: Partial<ReturnType<typeof useAuthStore>> = {}) {
    return {
        error: null as string | null,
        isLoading: false,
        isAuthenticated: false,
        login: vi.fn(),
        adoptSsoSession: vi.fn().mockResolvedValue(undefined),
        clearError: vi.fn(),
        ...overrides,
    }
}

async function mountPage() {
    const router = buildRouter()
    await router.isReady()
    const wrapper = mount(LoginPage, {
        global: {
            plugins: [router],
            stubs: { VMain: { template: '<div><slot /></div>' } },
        },
    })
    await wrapper.vm.$nextTick()
    return wrapper
}

describe('LoginPage single sign-on', () => {
    let assign: ReturnType<typeof vi.fn>

    beforeEach(() => {
        setActivePinia(createPinia())
        vi.clearAllMocks()

        capabilities.oidcEnabled = false

        vi.mocked(useAuthStore).mockReturnValue(
            buildAuthStoreMock() as ReturnType<typeof useAuthStore>
        )

        // jsdom refuses a real navigation, so the one thing this button must
        // do is observed rather than performed.
        assign = vi.fn()
        Object.defineProperty(window, 'location', {
            configurable: true,
            value: { ...window.location, assign, hash: '', pathname: '/login', search: '' },
        })
    })

    afterEach(() => {
        vi.restoreAllMocks()
    })

    it('renders the sign-in button when the server reports single sign-on enabled', async () => {
        capabilities.oidcEnabled = true
        const wrapper = await mountPage()

        expect(wrapper.find('[data-testid="sso-signin"]').exists()).toBe(true)
    })

    it('hides the button when single sign-on is disabled', async () => {
        const wrapper = await mountPage()

        expect(wrapper.find('[data-testid="sso-signin"]').exists()).toBe(false)
    })

    it('labels the button without naming the provider', async () => {
        // The capabilities endpoint is public and stays boolean-only, so the
        // interface never learns which provider the organisation uses.
        capabilities.oidcEnabled = true
        const wrapper = await mountPage()

        expect(wrapper.find('[data-testid="sso-signin"]').text()).toContain(
            'Sign in with single sign-on'
        )
    })

    it('navigates the browser to the start endpoint rather than fetching it', async () => {
        capabilities.oidcEnabled = true
        const wrapper = await mountPage()

        await wrapper.find('[data-testid="sso-signin"]').trigger('click')

        // `start` answers with a 302 to the identity provider. An XHR would
        // follow that in the background and fail on CORS instead of taking
        // the person to their provider, so this must be a full navigation.
        expect(assign).toHaveBeenCalledWith('/admin/api/v1/auth/oidc/start')
    })

    it('carries an intended destination through to the start endpoint', async () => {
        capabilities.oidcEnabled = true
        const router = buildRouter()
        await router.isReady()
        await router.push('/login?redirect=/workflows/42')

        const wrapper = mount(LoginPage, {
            global: {
                plugins: [router],
                stubs: { VMain: { template: '<div><slot /></div>' } },
            },
        })
        await wrapper.vm.$nextTick()

        await wrapper.find('[data-testid="sso-signin"]').trigger('click')

        expect(assign).toHaveBeenCalledWith(
            '/admin/api/v1/auth/oidc/start?return_to=%2Fworkflows%2F42'
        )
    })
})
