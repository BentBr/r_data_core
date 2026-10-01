import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import UserProfileMenu from './UserProfileMenu.vue'

const mockClearAuthState: Mock = vi.fn()
const mockLogout: Mock = vi.fn()
let mockUser: { username: string } | null = { username: 'alice' }
let mockIsSuperAdmin = false

vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        get user() {
            return mockUser
        },
        get isSuperAdmin() {
            return mockIsSuperAdmin
        },
        clearAuthState: mockClearAuthState,
        logout: mockLogout,
    }),
}))

let mockUserPreference: 'system' | 'light' | 'dark' = 'system'
const mockToggleTheme = vi.fn()
vi.mock('@/composables/useTheme', () => ({
    useTheme: () => ({
        isDark: false,
        toggleTheme: mockToggleTheme,
        get userPreference() {
            return { value: mockUserPreference }
        },
    }),
}))

const router = createRouter({
    history: createWebHistory(),
    routes: [
        { path: '/', component: { template: '<div />' } },
        { path: '/login', name: 'Login', component: { template: '<div />' } },
    ],
})

const mountMenu = () =>
    mount(UserProfileMenu, {
        global: { plugins: [router] },
    })

describe('UserProfileMenu', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockUser = { username: 'alice' }
        mockIsSuperAdmin = false
        mockUserPreference = 'system'
        mockLogout.mockResolvedValue(undefined)
    })

    it('renders the current username', () => {
        const wrapper = mountMenu()

        expect(wrapper.text()).toContain('alice')
    })

    it('shows "User" as the role for a non-super-admin', () => {
        mockIsSuperAdmin = false
        const wrapper = mountMenu()

        expect(wrapper.text()).toContain('User')
        expect(wrapper.text()).not.toContain('Super Admin')
    })

    it('shows "Super Admin" as the role for a super admin', () => {
        mockIsSuperAdmin = true
        const wrapper = mountMenu()

        expect(wrapper.text()).toContain('Super Admin')
    })

    it('renders the system theme label by default', () => {
        mockUserPreference = 'system'
        const wrapper = mountMenu()

        expect((wrapper.vm as any).getThemeDisplayName()).toBe('system')
    })

    it('renders the light theme label', () => {
        mockUserPreference = 'light'
        const wrapper = mountMenu()

        expect((wrapper.vm as any).getThemeDisplayName()).toBe('light')
    })

    it('renders the dark theme label', () => {
        mockUserPreference = 'dark'
        const wrapper = mountMenu()

        expect((wrapper.vm as any).getThemeDisplayName()).toBe('dark')
    })

    it('toggles the theme when the badge is invoked', async () => {
        const wrapper = mountMenu()

        await (wrapper.vm as any).toggleTheme()

        expect(mockToggleTheme).toHaveBeenCalled()
    })

    it('does not navigate when the profile option is clicked (not yet implemented)', () => {
        const wrapper = mountMenu()
        const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {})

        ;(wrapper.vm as any).goToProfile()

        expect(consoleSpy).toHaveBeenCalledWith('Profile page not yet implemented')
        consoleSpy.mockRestore()
    })

    it('clears auth state and redirects to the login route on logout', async () => {
        const pushSpy = vi.spyOn(router, 'push')
        const wrapper = mountMenu()

        await (wrapper.vm as any).handleLogout()

        expect(mockClearAuthState).toHaveBeenCalled()
        expect(pushSpy).toHaveBeenCalledWith({ name: 'Login', query: {} })
        expect(mockLogout).toHaveBeenCalled()
        pushSpy.mockRestore()
    })

    it('logs and continues when the logout request fails', async () => {
        mockLogout.mockRejectedValue(new Error('network error'))
        const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
        const wrapper = mountMenu()

        await (wrapper.vm as any).handleLogout()

        expect(mockClearAuthState).toHaveBeenCalled()
        expect(consoleErrorSpy).toHaveBeenCalledWith('Logout failed:', expect.any(Error))
        consoleErrorSpy.mockRestore()
    })
})
