import { describe, it, expect, vi, beforeEach, afterEach, type Mock } from 'vitest'
import { mount, DOMWrapper, type VueWrapper } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import { nextTick } from 'vue'
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

// The <v-menu> list is lazily rendered and sits behind the global
// <teleport> stub (test-setup.ts stubs teleport as `true`, which serializes
// teleported content as the literal string "[object Object]" instead of
// executing it). Opening the menu with a real teleport renders the list into
// document.body, which is the only way to exercise those template branches.
let mountedWrappers: VueWrapper[] = []

const mountMenuWithDom = () => {
    const wrapper = mount(UserProfileMenu, {
        global: { plugins: [router], stubs: { teleport: false } },
    })
    mountedWrappers.push(wrapper)
    return wrapper
}

const body = () => new DOMWrapper(document.body)

describe('UserProfileMenu', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockUser = { username: 'alice' }
        mockIsSuperAdmin = false
        mockUserPreference = 'system'
        mockLogout.mockResolvedValue(undefined)
    })

    afterEach(() => {
        mountedWrappers.forEach(w => w.unmount())
        mountedWrappers = []
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

    it('falls back to the system label for an unrecognised theme preference', () => {
        // userPreference is typed to the three known values; this exercises
        // the switch's defensive `default` branch for any other runtime value.
        mockUserPreference = 'unknown' as unknown as 'system'
        const wrapper = mountMenu()

        expect((wrapper.vm as any).getThemeDisplayName()).toBe('system')
    })

    describe('menu content (real teleport)', () => {
        it('renders the user info header, options, and logout row once opened', async () => {
            mockUser = { username: 'carol' }
            mockIsSuperAdmin = true
            const wrapper = mountMenuWithDom()

            await wrapper.find('button').trigger('click')
            await nextTick()
            await nextTick()

            const list = body()
            expect(list.text()).toContain('carol')
            expect(list.text()).toContain('Super Admin')
            expect(list.text()).toContain('profile')
            expect(list.text()).toContain('profile_subtitle')
            expect(list.text()).toContain('mode')
            expect(list.text()).toContain('system')
            expect(list.text()).toContain('logout')
            expect(list.text()).toContain('logout_subtitle')
        })

        it('disables the profile option (not yet implemented)', async () => {
            const wrapper = mountMenuWithDom()

            await wrapper.find('button').trigger('click')
            await nextTick()
            await nextTick()

            const profileItem = Array.from(document.querySelectorAll('.v-list-item')).find(el =>
                el.textContent?.includes('profile_subtitle')
            )
            expect(profileItem?.className).toContain('v-list-item--disabled')
        })

        it('triggers logout when the logout row is clicked', async () => {
            const pushSpy = vi.spyOn(router, 'push')
            const wrapper = mountMenuWithDom()

            await wrapper.find('button').trigger('click')
            await nextTick()
            await nextTick()

            const logoutItem = Array.from(document.querySelectorAll('.v-list-item')).find(el =>
                el.textContent?.includes('logout_subtitle')
            )
            expect(logoutItem).toBeTruthy()
            ;(logoutItem as HTMLElement).click()
            await nextTick()

            expect(mockClearAuthState).toHaveBeenCalled()
            expect(pushSpy).toHaveBeenCalledWith({ name: 'Login', query: {} })
            pushSpy.mockRestore()
        })
    })
})
