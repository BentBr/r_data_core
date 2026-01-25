import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import NoAccessPage from './NoAccessPage.vue'
import { useAuthStore } from '@/stores/auth'

// Mock dependencies
vi.mock('@/stores/auth')
vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (key: string) => {
            const translations: Record<string, string> = {
                'no_access.title': 'No Access',
                'no_access.message':
                    "You are logged in but don't have permission to access any features. Please contact your administrator.",
                'no_access.logged_in_as': 'Logged in as',
            }
            return translations[key] || key
        },
    }),
}))

const router = createRouter({
    history: createWebHistory(),
    routes: [{ path: '/no-access', component: NoAccessPage }],
})

describe('NoAccessPage', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    it('renders correctly', () => {
        const mockAuthStore = {
            user: {
                uuid: 'test-uuid',
                username: 'testuser',
                email: 'test@example.com',
                role_uuids: [],
                is_active: true,
                is_admin: false,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            },
        }
        vi.mocked(useAuthStore).mockReturnValue(mockAuthStore as any)

        const wrapper = mount(NoAccessPage, {
            global: {
                plugins: [router],
            },
        })

        expect(wrapper.text()).toContain('No Access')
        expect(wrapper.text()).toContain(
            "You are logged in but don't have permission to access any features"
        )
    })

    it('displays user information when user is available', () => {
        const mockAuthStore = {
            user: {
                uuid: 'test-uuid',
                username: 'testuser',
                email: 'test@example.com',
                role_uuids: [],
                is_active: true,
                is_admin: false,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            },
        }
        vi.mocked(useAuthStore).mockReturnValue(mockAuthStore as any)

        const wrapper = mount(NoAccessPage, {
            global: {
                plugins: [router],
            },
        })

        expect(wrapper.text()).toContain('Logged in as')
        expect(wrapper.text()).toContain('testuser')
    })

    it('shows message correctly', () => {
        const mockAuthStore = {
            user: {
                uuid: 'test-uuid',
                username: 'testuser',
                email: 'test@example.com',
                role_uuids: [],
                is_active: true,
                is_admin: false,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            },
        }
        vi.mocked(useAuthStore).mockReturnValue(mockAuthStore as any)

        const wrapper = mount(NoAccessPage, {
            global: {
                plugins: [router],
            },
        })

        const message = wrapper.text()
        expect(message).toContain("You are logged in but don't have permission")
        expect(message).toContain('Please contact your administrator')
    })

    it('uses translations correctly', () => {
        const mockAuthStore = {
            user: {
                uuid: 'test-uuid',
                username: 'testuser',
                email: 'test@example.com',
                role_uuids: [],
                is_active: true,
                is_admin: false,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            },
        }
        vi.mocked(useAuthStore).mockReturnValue(mockAuthStore as any)

        const wrapper = mount(NoAccessPage, {
            global: {
                plugins: [router],
            },
        })

        // Check that translation keys are used
        expect(wrapper.text()).toContain('No Access')
        expect(wrapper.text()).toContain('Logged in as')
    })
})
