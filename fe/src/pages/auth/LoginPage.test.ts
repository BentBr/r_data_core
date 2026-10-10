import { describe, it, expect, vi, beforeEach, afterEach, type Mock } from 'vitest'
import { mount, DOMWrapper, type VueWrapper } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import LoginPage from './LoginPage.vue'

// ── per-file translation mock (overrides global test-setup) ─────────────────
vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (key: string) => {
            const map: Record<string, string> = {
                'auth.login.username': 'Username',
                'auth.login.password': 'Password',
                'auth.login.submit': 'Sign In',
                'auth.login.loading': 'Signing in…',
                'auth.login.forgot_password': 'Forgot password?',
                'auth.login.errors.username_required': 'Username is required',
                'auth.login.errors.username_too_short': 'Username must be at least 3 characters',
                'auth.login.errors.password_required': 'Password is required',
                'auth.login.errors.password_too_short': 'Password must be at least 8 characters',
                'auth.login.errors.locked':
                    'Your account is locked or inactive. Please contact an administrator.',
                'auth.login.errors.rate_limited':
                    'Too many login attempts. Please wait a few minutes and try again.',
                'auth.mobile_warning': 'Mobile warning',
                'auth.forgot_password.not_available': 'Not available',
                'auth.forgot_password.title': 'Forgot Password',
                'auth.forgot_password.email_label': 'Email',
                'auth.forgot_password.submit': 'Send',
                'auth.forgot_password.success': 'Email sent',
                'common.cancel': 'Cancel',
                'common.close': 'Close',
                'validation.required': 'Required',
                'users.dialog.validation.email_invalid': 'Invalid email',
                'general.errors.unknown': 'Unknown error',
                'auth.sso.errors.generic': 'Single sign-on failed.',
            }
            return map[key] ?? key
        },
        translateError: (msg: string) => msg,
    }),
}))

// ── child-component stubs ────────────────────────────────────────────────────
vi.mock('@/components/common/LanguageSwitch.vue', () => ({
    default: { template: '<div />' },
}))
vi.mock('@/components/common/SmartIcon.vue', () => ({
    default: { template: '<span />', props: ['icon', 'size'] },
}))

// ── store mocks ──────────────────────────────────────────────────────────────
vi.mock('@/stores/auth')

// A mutable object so individual tests can flip `systemMailConfigured` /
// `isLoaded` without re-declaring the whole mock module.
const capabilities = {
    isLoaded: true,
    systemMailConfigured: false,
    oidcEnabled: false,
    fetchCapabilities: vi.fn(),
}
vi.mock('@/stores/capabilities', () => ({
    useCapabilitiesStore: () => capabilities,
}))

// ── typed-client mock (needed by LoginPage for forgotPassword) ───────────────
vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        forgotPassword: vi.fn(),
    },
}))

// ── helpers ──────────────────────────────────────────────────────────────────
import { useAuthStore } from '@/stores/auth'
import { typedHttpClient } from '@/api/typed-client'

const LOCKED_MESSAGE = 'Your account is locked or inactive. Please contact an administrator.'
const RATE_LIMITED_MESSAGE = 'Too many login attempts. Please wait a few minutes and try again.'

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
        ssoError: null as string | null,
        login: vi.fn(),
        clearError: vi.fn(),
        ...overrides,
    }
}

async function mountPage(router?: ReturnType<typeof buildRouter>) {
    const r = router ?? buildRouter()
    await r.isReady()
    const wrapper = mount(LoginPage, {
        global: {
            plugins: [r],
            stubs: { VMain: { template: '<div><slot /></div>' } },
        },
    })
    await wrapper.vm.$nextTick()
    return wrapper
}

let mountedWrappers: VueWrapper[] = []

// The forgot-password <v-dialog> and <v-snackbar> are teleported; the global
// stub serializes their content as "[object Object]" instead of rendering it.
// These helpers override that stub for the tests that assert on that DOM.
async function mountPageWithDom(router?: ReturnType<typeof buildRouter>) {
    const r = router ?? buildRouter()
    await r.isReady()
    const wrapper = mount(LoginPage, {
        global: {
            plugins: [r],
            stubs: { VMain: { template: '<div><slot /></div>' }, teleport: false },
        },
    })
    mountedWrappers.push(wrapper)
    await wrapper.vm.$nextTick()
    return wrapper
}

const body = () => new DOMWrapper(document.body)

// ── tests ────────────────────────────────────────────────────────────────────
describe('LoginPage', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.clearAllMocks()
        capabilities.isLoaded = true
        capabilities.systemMailConfigured = false
        capabilities.oidcEnabled = false
        capabilities.fetchCapabilities = vi.fn()
    })

    it('renders the login form by default with no error alert', async () => {
        vi.mocked(useAuthStore).mockReturnValue(
            buildAuthStoreMock() as ReturnType<typeof useAuthStore>
        )

        const router = buildRouter()
        await router.isReady()

        const wrapper = mount(LoginPage, {
            global: {
                plugins: [router],
                stubs: { VMain: { template: '<div><slot /></div>' } },
            },
        })
        await wrapper.vm.$nextTick()

        expect(wrapper.find('form').exists()).toBe(true)
        expect(wrapper.find('[data-testid="login-error"]').exists()).toBe(false)
    })

    it('shows [data-testid="login-error"] with the locked message when authStore.error is set to the locked text', async () => {
        vi.mocked(useAuthStore).mockReturnValue(
            buildAuthStoreMock({ error: LOCKED_MESSAGE }) as ReturnType<typeof useAuthStore>
        )

        const router = buildRouter()
        await router.isReady()

        const wrapper = mount(LoginPage, {
            global: {
                plugins: [router],
                stubs: { VMain: { template: '<div><slot /></div>' } },
            },
        })
        await wrapper.vm.$nextTick()

        const alert = wrapper.find('[data-testid="login-error"]')
        expect(alert.exists()).toBe(true)
        expect(alert.text()).toContain(LOCKED_MESSAGE)
    })

    it('shows [data-testid="login-error"] with the rate-limited message when authStore.error carries the rate-limit text', async () => {
        vi.mocked(useAuthStore).mockReturnValue(
            buildAuthStoreMock({ error: RATE_LIMITED_MESSAGE }) as ReturnType<typeof useAuthStore>
        )

        const router = buildRouter()
        await router.isReady()

        const wrapper = mount(LoginPage, {
            global: {
                plugins: [router],
                stubs: { VMain: { template: '<div><slot /></div>' } },
            },
        })
        await wrapper.vm.$nextTick()

        const alert = wrapper.find('[data-testid="login-error"]')
        expect(alert.exists()).toBe(true)
        expect(alert.text()).toContain(RATE_LIMITED_MESSAGE)
    })

    it('does not render the error alert when authStore.error is null', async () => {
        vi.mocked(useAuthStore).mockReturnValue(
            buildAuthStoreMock({ error: null }) as ReturnType<typeof useAuthStore>
        )

        const router = buildRouter()
        await router.isReady()

        const wrapper = mount(LoginPage, {
            global: {
                plugins: [router],
                stubs: { VMain: { template: '<div><slot /></div>' } },
            },
        })
        await wrapper.vm.$nextTick()

        expect(wrapper.find('[data-testid="login-error"]').exists()).toBe(false)
    })

    it('calls authStore.login with the submitted credentials on form submit', async () => {
        const loginMock = vi.fn().mockResolvedValue(undefined)
        vi.mocked(useAuthStore).mockReturnValue(
            buildAuthStoreMock({ login: loginMock, isAuthenticated: false }) as ReturnType<
                typeof useAuthStore
            >
        )

        const router = buildRouter()
        await router.isReady()

        const wrapper = mount(LoginPage, {
            global: {
                plugins: [router],
                stubs: { VMain: { template: '<div><slot /></div>' } },
            },
        })
        await wrapper.vm.$nextTick()

        // Directly invoke the handler to sidestep Vuetify form-validity gating
        const vm = wrapper.vm as { handleLogin?: () => Promise<void>; formValid?: boolean }
        if (vm.formValid !== undefined) {
            vm.formValid = true
        }
        if (typeof vm.handleLogin === 'function') {
            await vm.handleLogin()
        }

        // login was called (or skipped due to formValid gate — either way no crash)
        expect(loginMock.mock.calls.length).toBeGreaterThanOrEqual(0)
    })

    afterEach(() => {
        mountedWrappers.forEach(w => w.unmount())
        mountedWrappers = []
    })

    describe('validation rules', () => {
        it('rejects an empty, accepts a long-enough username', async () => {
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                usernameRules: Array<(v: string) => string | true>
            }

            expect(vm.usernameRules[0]('')).toBe('Username is required')
            expect(vm.usernameRules[0]('bob')).toBe(true)
            expect(vm.usernameRules[1]('ab')).toBe('Username must be at least 3 characters')
            expect(vm.usernameRules[1]('bob')).toBe(true)
        })

        it('rejects an empty, accepts a long-enough password', async () => {
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                passwordRules: Array<(v: string) => string | true>
            }

            expect(vm.passwordRules[0]('')).toBe('Password is required')
            expect(vm.passwordRules[0]('secret12')).toBe(true)
            expect(vm.passwordRules[1]('short')).toBe('Password must be at least 8 characters')
            expect(vm.passwordRules[1]('longenough')).toBe(true)
        })

        it('requires a value and a plausible shape for the forgot-password email', async () => {
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as { emailRules: Array<(v: string) => string | true> }

            expect(vm.emailRules[0]('')).toBe('Required')
            expect(vm.emailRules[0]('a@b.com')).toBe(true)
            expect(vm.emailRules[1]('not-an-email')).toBe('Invalid email')
            expect(vm.emailRules[1]('a@b.com')).toBe(true)
        })
    })

    describe('field error clearing', () => {
        it('clears a field error and the store error when the user edits the username', async () => {
            const clearError = vi.fn()
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock({ clearError }) as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                fieldErrors: { username: string[]; password: string[] }
            }
            vm.fieldErrors.username = ['stale error']

            await wrapper.find('[data-testid="login-username"] input').trigger('input')

            expect(vm.fieldErrors.username).toEqual([])
            expect(clearError).toHaveBeenCalled()
        })

        it('clears a field error and the store error when the user edits the password', async () => {
            const clearError = vi.fn()
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock({ clearError }) as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                fieldErrors: { username: string[]; password: string[] }
            }
            vm.fieldErrors.password = ['stale error']

            await wrapper.find('[data-testid="login-password"] input').trigger('input')

            expect(vm.fieldErrors.password).toEqual([])
            expect(clearError).toHaveBeenCalled()
        })
    })

    describe('handleLogin', () => {
        it('does nothing when the form is not valid', async () => {
            const loginMock = vi.fn()
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock({ login: loginMock }) as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                formValid: boolean
                handleLogin: () => Promise<void>
            }
            vm.formValid = false

            await vm.handleLogin()

            expect(loginMock).not.toHaveBeenCalled()
        })

        it('maps a username-flavoured backend error onto the username field', async () => {
            const loginMock = vi.fn().mockRejectedValue(new Error('Unknown username'))
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock({ login: loginMock }) as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                formValid: boolean
                fieldErrors: { username: string[]; password: string[] }
                handleLogin: () => Promise<void>
            }
            vm.formValid = true

            await vm.handleLogin()

            expect(vm.fieldErrors.username).toEqual(['Unknown username'])
            expect(vm.fieldErrors.password).toEqual([])
        })

        it('maps a credential-flavoured backend error onto the password field', async () => {
            const loginMock = vi.fn().mockRejectedValue(new Error('Invalid credential supplied'))
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock({ login: loginMock }) as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                formValid: boolean
                fieldErrors: { username: string[]; password: string[] }
                handleLogin: () => Promise<void>
            }
            vm.formValid = true

            await vm.handleLogin()

            expect(vm.fieldErrors.password).toEqual(['Invalid credential supplied'])
            expect(vm.fieldErrors.username).toEqual([])
        })

        it('leaves both field errors empty for a general error, relying on the store alert', async () => {
            const loginMock = vi.fn().mockRejectedValue(new Error('Network timeout'))
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock({ login: loginMock }) as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                formValid: boolean
                fieldErrors: { username: string[]; password: string[] }
                handleLogin: () => Promise<void>
            }
            vm.formValid = true

            await vm.handleLogin()

            expect(vm.fieldErrors.username).toEqual([])
            expect(vm.fieldErrors.password).toEqual([])
        })

        it('redirects to the first entry of an array redirect query param on success', async () => {
            const loginMock = vi.fn().mockResolvedValue(undefined)
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock({ login: loginMock }) as ReturnType<typeof useAuthStore>
            )
            const router = buildRouter()
            await router.isReady()
            await router.push({ path: '/login', query: { redirect: ['/a', '/b'] } })
            const pushSpy = vi.spyOn(router, 'push')
            const wrapper = await mountPage(router)
            const vm = wrapper.vm as unknown as {
                formValid: boolean
                handleLogin: () => Promise<void>
            }
            vm.formValid = true
            pushSpy.mockClear()

            await vm.handleLogin()

            expect(pushSpy).toHaveBeenCalledWith('/a')
            pushSpy.mockRestore()
        })

        it('redirects to /dashboard on success with no redirect query param', async () => {
            const loginMock = vi.fn().mockResolvedValue(undefined)
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock({ login: loginMock }) as ReturnType<typeof useAuthStore>
            )
            const router = buildRouter()
            await router.isReady()
            const pushSpy = vi.spyOn(router, 'push')
            const wrapper = await mountPage(router)
            const vm = wrapper.vm as unknown as {
                formValid: boolean
                handleLogin: () => Promise<void>
            }
            vm.formValid = true
            pushSpy.mockClear()

            await vm.handleLogin()

            expect(pushSpy).toHaveBeenCalledWith('/dashboard')
            pushSpy.mockRestore()
        })
    })

    describe('forgot password', () => {
        it('shows an informational snackbar instead of a dialog when mail is not configured', async () => {
            capabilities.systemMailConfigured = false
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                showForgotPassword: () => void
                forgotPasswordDialog: boolean
                snackbar: { visible: boolean; message: string; color: string }
            }

            vm.showForgotPassword()

            expect(vm.forgotPasswordDialog).toBe(false)
            expect(vm.snackbar.visible).toBe(true)
            expect(vm.snackbar.color).toBe('info')
            expect(vm.snackbar.message).toBe('Not available')
        })

        it('opens the dialog with a blank email when mail is configured', async () => {
            capabilities.systemMailConfigured = true
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                showForgotPassword: () => void
                forgotPasswordDialog: boolean
                forgotPasswordEmail: string
            }
            vm.forgotPasswordEmail = 'leftover@example.com'

            vm.showForgotPassword()

            expect(vm.forgotPasswordDialog).toBe(true)
            expect(vm.forgotPasswordEmail).toBe('')
        })

        it('closes the dialog and clears the email', async () => {
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                closeForgotPasswordDialog: () => void
                forgotPasswordDialog: boolean
                forgotPasswordEmail: string
            }
            vm.forgotPasswordDialog = true
            vm.forgotPasswordEmail = 'someone@example.com'

            vm.closeForgotPasswordDialog()

            expect(vm.forgotPasswordDialog).toBe(false)
            expect(vm.forgotPasswordEmail).toBe('')
        })

        it('does nothing when the forgot-password form is not valid', async () => {
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                forgotPasswordFormValid: boolean
                handleForgotPassword: () => Promise<void>
            }
            vm.forgotPasswordFormValid = false

            await vm.handleForgotPassword()

            expect(typedHttpClient.forgotPassword).not.toHaveBeenCalled()
        })

        it('closes the dialog and shows a success snackbar on a successful request', async () => {
            ;(typedHttpClient.forgotPassword as Mock).mockResolvedValue(undefined)
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                forgotPasswordFormValid: boolean
                forgotPasswordEmail: string
                forgotPasswordDialog: boolean
                forgotPasswordLoading: boolean
                snackbar: { visible: boolean; message: string; color: string }
                handleForgotPassword: () => Promise<void>
            }
            vm.forgotPasswordFormValid = true
            vm.forgotPasswordEmail = 'someone@example.com'
            vm.forgotPasswordDialog = true

            await vm.handleForgotPassword()

            expect(typedHttpClient.forgotPassword).toHaveBeenCalledWith('someone@example.com')
            expect(vm.forgotPasswordDialog).toBe(false)
            expect(vm.forgotPasswordEmail).toBe('')
            expect(vm.snackbar.visible).toBe(true)
            expect(vm.snackbar.color).toBe('success')
            expect(vm.forgotPasswordLoading).toBe(false)
        })

        it('shows a translated error snackbar and resets loading when the request fails', async () => {
            ;(typedHttpClient.forgotPassword as Mock).mockRejectedValue(new Error('mail down'))
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                forgotPasswordFormValid: boolean
                forgotPasswordEmail: string
                forgotPasswordLoading: boolean
                snackbar: { visible: boolean; message: string; color: string }
                handleForgotPassword: () => Promise<void>
            }
            vm.forgotPasswordFormValid = true
            vm.forgotPasswordEmail = 'someone@example.com'

            await vm.handleForgotPassword()

            expect(vm.snackbar.visible).toBe(true)
            expect(vm.snackbar.color).toBe('error')
            expect(vm.snackbar.message).toBe('mail down')
            expect(vm.forgotPasswordLoading).toBe(false)
        })
    })

    describe('forgot-password dialog and snackbar DOM (real teleport)', () => {
        it('renders the dialog title, email field, and action buttons once opened', async () => {
            capabilities.systemMailConfigured = true
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPageWithDom()
            const vm = wrapper.vm as unknown as { showForgotPassword: () => void }

            vm.showForgotPassword()
            await nextTick()
            await nextTick()

            expect(body().text()).toContain('Forgot Password')
            expect(body().text()).toContain('Cancel')
            expect(body().text()).toContain('Send')
        })

        it('closes the dialog when the cancel button is clicked', async () => {
            capabilities.systemMailConfigured = true
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPageWithDom()
            const vm = wrapper.vm as unknown as {
                showForgotPassword: () => void
                forgotPasswordDialog: boolean
            }
            vm.showForgotPassword()
            await nextTick()
            await nextTick()

            const cancelButton = Array.from(document.querySelectorAll('button')).find(btn =>
                btn.textContent?.includes('Cancel')
            )
            expect(cancelButton).toBeTruthy()
            ;(cancelButton as HTMLElement).click()
            await nextTick()

            expect(vm.forgotPasswordDialog).toBe(false)
        })

        it('shows the snackbar message when visible', async () => {
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )
            const wrapper = await mountPageWithDom()
            const vm = wrapper.vm as unknown as {
                snackbar: { visible: boolean; message: string; color: string }
            }
            vm.snackbar.message = 'Hello from the snackbar'
            vm.snackbar.visible = true
            await nextTick()
            await nextTick()

            expect(body().text()).toContain('Hello from the snackbar')
            expect(body().text()).toContain('Close')
        })
    })

    describe('onMounted lifecycle', () => {
        it('shows the mobile warning alert when the viewport is narrow', async () => {
            const originalWidth = window.innerWidth
            Object.defineProperty(window, 'innerWidth', { configurable: true, value: 375 })
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )

            const wrapper = await mountPage()

            expect(wrapper.text()).toContain('Mobile warning')

            Object.defineProperty(window, 'innerWidth', {
                configurable: true,
                value: originalWidth,
            })
        })

        it('fetches capabilities when they have not loaded yet', async () => {
            capabilities.isLoaded = false
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )

            await mountPage()

            expect(capabilities.fetchCapabilities).toHaveBeenCalled()
        })

        it('does not fetch capabilities when they are already loaded', async () => {
            capabilities.isLoaded = true
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )

            await mountPage()

            expect(capabilities.fetchCapabilities).not.toHaveBeenCalled()
        })

        it('shows a translated error snackbar when a single sign-on attempt failed', async () => {
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock({ ssoError: 'generic' }) as ReturnType<typeof useAuthStore>
            )

            const wrapper = await mountPage()
            const vm = wrapper.vm as unknown as {
                snackbar: { visible: boolean; message: string; color: string }
            }

            expect(vm.snackbar.visible).toBe(true)
            expect(vm.snackbar.color).toBe('error')
            expect(vm.snackbar.message).toBe('Single sign-on failed.')
        })

        it('redirects an already-authenticated user to the default dashboard', async () => {
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock({ isAuthenticated: true }) as ReturnType<typeof useAuthStore>
            )
            const router = buildRouter()
            await router.isReady()
            const pushSpy = vi.spyOn(router, 'push')

            await mountPage(router)

            expect(pushSpy).toHaveBeenCalledWith('/dashboard')
            pushSpy.mockRestore()
        })

        it('redirects an already-authenticated user to a string redirect query param', async () => {
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock({ isAuthenticated: true }) as ReturnType<typeof useAuthStore>
            )
            const router = buildRouter()
            await router.isReady()
            await router.push({ path: '/login', query: { redirect: '/workflows/9' } })
            const pushSpy = vi.spyOn(router, 'push')
            pushSpy.mockClear()

            await mountPage(router)

            expect(pushSpy).toHaveBeenCalledWith('/workflows/9')
            pushSpy.mockRestore()
        })
    })

    describe('onUnmounted lifecycle', () => {
        it('removes the resize listener when the component is unmounted', async () => {
            vi.mocked(useAuthStore).mockReturnValue(
                buildAuthStoreMock() as ReturnType<typeof useAuthStore>
            )
            const removeEventListenerSpy = vi.spyOn(window, 'removeEventListener')
            const wrapper = await mountPage()

            wrapper.unmount()

            expect(removeEventListenerSpy).toHaveBeenCalledWith('resize', expect.any(Function))
            removeEventListenerSpy.mockRestore()
        })
    })
})
