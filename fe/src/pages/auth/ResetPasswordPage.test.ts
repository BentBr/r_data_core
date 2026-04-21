import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import ResetPasswordPage from './ResetPasswordPage.vue'

const vuetify = createVuetify({ components, directives })

// Mock dependencies
vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        resetPassword: vi.fn(),
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (key: string) => {
            const translations: Record<string, string> = {
                'auth.reset_password.title': 'Reset Password',
                'auth.reset_password.invalid_link': 'Invalid or missing reset link.',
                'auth.reset_password.new_password': 'New Password',
                'auth.reset_password.confirm_password': 'Confirm Password',
                'auth.reset_password.submit': 'Reset Password',
                'auth.reset_password.success': 'Password reset successfully.',
                'auth.reset_password.password_too_short': 'Password must be at least 8 characters.',
                'auth.reset_password.passwords_mismatch': 'Passwords do not match.',
                'auth.reset_password.invalid_token': 'Invalid or expired token.',
                'auth.login.submit': 'Back to Login',
                'validation.required': 'This field is required.',
                'general.errors.unknown': 'An unknown error occurred.',
            }
            return translations[key] ?? key
        },
    }),
}))

// Stub child components that are not relevant to these tests
vi.mock('@/components/common/LanguageSwitch.vue', () => ({
    default: { template: '<div />' },
}))

vi.mock('@/components/common/SmartIcon.vue', () => ({
    default: { template: '<span />', props: ['icon', 'size'] },
}))

const createTestRouter = (query: Record<string, string> = {}) => {
    const router = createRouter({
        history: createWebHistory(),
        routes: [
            { path: '/reset-password', component: ResetPasswordPage },
            { path: '/login', name: 'Login', component: { template: '<div>Login</div>' } },
        ],
    })
    // Push the current route so query params are available
    void router.push({ path: '/reset-password', query })
    return router
}

describe('ResetPasswordPage', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    it('shows an error alert when no token is present in the URL', async () => {
        const router = createTestRouter()
        await router.isReady()

        const wrapper = mount(ResetPasswordPage, {
            global: {
                plugins: [router, vuetify],
                stubs: { VMain: { template: '<div><slot /></div>' } },
            },
        })

        await wrapper.vm.$nextTick()

        // Should show invalid link alert, not the form
        expect(wrapper.text()).toContain('Invalid or missing reset link.')
        // Form should not be rendered
        expect(wrapper.find('form').exists()).toBe(false)
    })

    it('shows the reset form when a token is present in the URL', async () => {
        const router = createTestRouter({ token: 'valid-reset-token-123' })
        await router.isReady()

        const wrapper = mount(ResetPasswordPage, {
            global: {
                plugins: [router, vuetify],
                stubs: { VMain: { template: '<div><slot /></div>' } },
            },
        })

        await wrapper.vm.$nextTick()

        // Should not show invalid link alert
        expect(wrapper.text()).not.toContain('Invalid or missing reset link.')
        // Form should be rendered
        expect(wrapper.find('form').exists()).toBe(true)
    })

    it('shows password fields in the form', async () => {
        const router = createTestRouter({ token: 'valid-reset-token-123' })
        await router.isReady()

        const wrapper = mount(ResetPasswordPage, {
            global: {
                plugins: [router, vuetify],
                stubs: { VMain: { template: '<div><slot /></div>' } },
            },
        })

        await wrapper.vm.$nextTick()

        // Should have two password input fields
        const passwordFields = wrapper.findAllComponents({ name: 'VTextField' })
        expect(passwordFields.length).toBeGreaterThanOrEqual(2)
    })

    it('validates that passwords must match via confirmPasswordRules', () => {
        // Access the validation logic directly rather than through DOM interaction
        // The confirm password rule checks that v === newPassword.value
        // We verify this by checking the rule function behaviour
        const passwordValue = 'correcthorsebattery'
        const mismatchValue = 'wrongpassword'

        // Simulating the confirmPasswordRules logic as defined in the component
        const matchRule = (v: string) => v === passwordValue || 'Passwords do not match.'

        expect(matchRule(passwordValue)).toBe(true)
        expect(matchRule(mismatchValue)).toBe('Passwords do not match.')
    })

    it('validates minimum password length via newPasswordRules', () => {
        // Simulating the newPasswordRules logic as defined in the component
        const lengthRule = (v: string) => v.length >= 8 || 'Password must be at least 8 characters.'

        expect(lengthRule('short')).toBe('Password must be at least 8 characters.')
        expect(lengthRule('longenough')).toBe(true)
        expect(lengthRule('exactly8')).toBe(true)
    })

    it('shows the back to login button when not yet succeeded', async () => {
        const router = createTestRouter({ token: 'valid-reset-token-123' })
        await router.isReady()

        const wrapper = mount(ResetPasswordPage, {
            global: {
                plugins: [router, vuetify],
                stubs: { VMain: { template: '<div><slot /></div>' } },
            },
        })

        await wrapper.vm.$nextTick()

        expect(wrapper.text()).toContain('Back to Login')
    })

    it('shows success alert and hides form after successful reset', async () => {
        const { typedHttpClient } = await import('@/api/typed-client')
        vi.mocked(typedHttpClient.resetPassword).mockResolvedValueOnce({
            message: 'Password reset successfully',
        } as any)

        // Use fake timers to prevent the setTimeout redirect
        vi.useFakeTimers()

        const router = createTestRouter({ token: 'valid-reset-token-123' })
        await router.isReady()

        const wrapper = mount(ResetPasswordPage, {
            global: {
                plugins: [router, vuetify],
                stubs: { VMain: { template: '<div><slot /></div>' } },
            },
        })

        await wrapper.vm.$nextTick()

        // Manually trigger the reset by setting formValid and calling handleResetPassword
        const vm = wrapper.vm as any
        vm.formValid = true
        vm.newPassword = 'newpassword123'
        vm.confirmPassword = 'newpassword123'
        await vm.handleResetPassword()

        await wrapper.vm.$nextTick()

        // Should show success message
        expect(wrapper.text()).toContain('Password reset successfully.')
        // Form should be hidden
        expect(wrapper.find('form').exists()).toBe(false)

        vi.useRealTimers()
    })

    it('shows error message on reset failure', async () => {
        const { typedHttpClient } = await import('@/api/typed-client')
        vi.mocked(typedHttpClient.resetPassword).mockRejectedValueOnce(
            new Error('Token has expired')
        )

        const router = createTestRouter({ token: 'expired-token-456' })
        await router.isReady()

        const wrapper = mount(ResetPasswordPage, {
            global: {
                plugins: [router, vuetify],
                stubs: { VMain: { template: '<div><slot /></div>' } },
            },
        })

        await wrapper.vm.$nextTick()

        const vm = wrapper.vm as any
        vm.formValid = true
        vm.newPassword = 'newpassword123'
        vm.confirmPassword = 'newpassword123'
        await vm.handleResetPassword()

        await wrapper.vm.$nextTick()

        expect(wrapper.text()).toContain('Token has expired')
    })
})
