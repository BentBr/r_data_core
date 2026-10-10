import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import MainLayout from './MainLayout.vue'

const mockCanAccessRoute: Mock = vi.fn(() => true)
let mockUser: { username: string } | null = { username: 'alice' }

vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        get user() {
            return mockUser
        },
        canAccessRoute: mockCanAccessRoute,
    }),
}))

vi.mock('@/stores/versions', () => ({
    useVersionStore: () => ({
        feVersion: '1.2.3',
        coreVersion: '4.5.6',
        workerVersion: { version: '7.8.9' },
        maintenanceVersion: null,
    }),
}))

const stubs = {
    LanguageSwitch: { template: '<div data-testid="language-switch-stub" />' },
    UserProfileMenu: { template: '<div data-testid="user-profile-menu-stub" />' },
    DefaultPasswordBanner: { template: '<div />' },
    LicenseBanner: { template: '<div />' },
    MobileWarningBanner: { template: '<div />' },
}

const router = createRouter({
    history: createWebHistory(),
    routes: [
        { path: '/dashboard', component: { template: '<div>Dashboard</div>' } },
        { path: '/entity-definitions', component: { template: '<div>Entity Defs</div>' } },
        { path: '/entities', component: { template: '<div>Entities</div>' } },
        { path: '/api-keys', component: { template: '<div>API Keys</div>' } },
        { path: '/workflows', component: { template: '<div>Workflows</div>' } },
        { path: '/permissions', component: { template: '<div>Permissions</div>' } },
        { path: '/system', component: { template: '<div>System</div>' } },
    ],
})

// VNavigationDrawer/VAppBar need an injected Vuetify layout, which only a
// `v-app` ancestor provides — mount through a thin wrapper that supplies one.
const AppWrapper = {
    components: { MainLayout },
    template: '<v-app><MainLayout /></v-app>',
}

const mountLayout = async (path = '/dashboard') => {
    await router.push(path)
    await router.isReady()
    const wrapper = mount(AppWrapper, {
        global: { plugins: [router], stubs },
    })
    await wrapper.vm.$nextTick()
    return wrapper.findComponent(MainLayout)
}

describe('MainLayout', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockUser = { username: 'alice' }
        mockCanAccessRoute.mockReturnValue(true)
        Object.defineProperty(window, 'innerWidth', {
            writable: true,
            configurable: true,
            value: 1400,
        })
    })

    it('renders a navigation item for every route the user can access', async () => {
        const wrapper = await mountLayout()

        expect(wrapper.find('[data-testid="nav-item-/dashboard"]').exists()).toBe(true)
        expect(wrapper.find('[data-testid="nav-item-/system"]').exists()).toBe(true)
        expect(mockCanAccessRoute).toHaveBeenCalledWith('/dashboard')
    })

    it('filters out navigation items the user cannot access', async () => {
        mockCanAccessRoute.mockImplementation((path: string) => path === '/dashboard')

        const wrapper = await mountLayout()

        expect(wrapper.find('[data-testid="nav-item-/dashboard"]').exists()).toBe(true)
        expect(wrapper.find('[data-testid="nav-item-/system"]').exists()).toBe(false)
        expect(wrapper.find('[data-testid="nav-item-/permissions"]').exists()).toBe(false)
    })

    it('shows the user profile menu when a user is signed in', async () => {
        mockUser = { username: 'alice' }
        const wrapper = await mountLayout()

        expect(wrapper.find('[data-testid="user-profile-menu-stub"]').exists()).toBe(true)
    })

    it('hides the user profile menu when no user is signed in', async () => {
        mockUser = null
        const wrapper = await mountLayout()

        expect(wrapper.find('[data-testid="user-profile-menu-stub"]').exists()).toBe(false)
    })

    it('shows the matching navigation title as the page title', async () => {
        const wrapper = await mountLayout('/system')

        expect(wrapper.text()).toContain('system')
    })

    it('falls back to the default title when the route has no matching navigation item', async () => {
        const wrapper = await mountLayout('/unknown-route-outside-nav')

        expect((wrapper.vm as any).currentPageTitle).toBe('R Data Core')
        await router.push('/dashboard')
    })

    it('renders the version numbers from the version store', async () => {
        const wrapper = await mountLayout()

        expect(wrapper.text()).toContain('1.2.3')
        expect(wrapper.text()).toContain('4.5.6')
        expect(wrapper.text()).toContain('7.8.9')
    })

    it('toggles the navigation drawer open state', async () => {
        const wrapper = await mountLayout()
        const initial = (wrapper.vm as any).drawer

        ;(wrapper.vm as any).toggleNav()

        expect((wrapper.vm as any).drawer).toBe(!initial)
    })

    it('opens the drawer by default on desktop widths', async () => {
        Object.defineProperty(window, 'innerWidth', {
            writable: true,
            configurable: true,
            value: 1400,
        })
        const wrapper = await mountLayout()

        expect((wrapper.vm as any).drawer).toBe(true)
        expect((wrapper.vm as any).isMobile).toBe(false)
    })

    it('closes the drawer by default on mobile widths', async () => {
        Object.defineProperty(window, 'innerWidth', {
            writable: true,
            configurable: true,
            value: 800,
        })
        const wrapper = await mountLayout()

        expect((wrapper.vm as any).drawer).toBe(false)
        expect((wrapper.vm as any).isMobile).toBe(true)
    })

    it('reacts to window resize events while mounted', async () => {
        const wrapper = await mountLayout()
        expect((wrapper.vm as any).isMobile).toBe(false)

        Object.defineProperty(window, 'innerWidth', {
            writable: true,
            configurable: true,
            value: 500,
        })
        window.dispatchEvent(new Event('resize'))
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).isMobile).toBe(true)
        expect((wrapper.vm as any).drawer).toBe(false)
    })

    it('removes the resize listener on unmount', async () => {
        await router.push('/dashboard')
        await router.isReady()
        const removeSpy = vi.spyOn(window, 'removeEventListener')
        const root = mount(AppWrapper, { global: { plugins: [router], stubs } })
        await root.vm.$nextTick()

        root.unmount()

        expect(removeSpy).toHaveBeenCalledWith('resize', expect.any(Function))
        removeSpy.mockRestore()
    })
})
