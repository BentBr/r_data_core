import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import DashboardPage from './DashboardPage.vue'
import { useAuthStore } from '@/stores/auth'
import { typedHttpClient } from '@/api/typed-client'
import type { DashboardStats } from '@/api/clients/meta'

// Mock dependencies
vi.mock('@/stores/auth')
vi.mock('@/api/typed-client')
vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (key: string, params?: Record<string, string | number>) => {
            const translations: Record<string, string> = {
                'dashboard.title': 'R Data Core Admin Dashboard',
                'dashboard.tiles.entity_definitions': 'Entity Definitions',
                'dashboard.tiles.entities': 'Entities',
                'dashboard.tiles.workflows': 'Workflows',
                'dashboard.tiles.online_users': 'Online Users',
                'dashboard.tiles.top_entity_type': `Top: ${params?.type ?? ''} (${params?.count ?? 0})`,
                'dashboard.tiles.latest_workflow_states': `Latest: ${Array.isArray(params?.states) ? params.states.join(', ') : (params?.states ?? '')}`,
                'dashboard.quick_actions.title': 'Quick Actions',
                'dashboard.quick_actions.new_entity_definition': 'New Entity Definition',
                'dashboard.quick_actions.create_entity': 'Create Entity',
                'dashboard.quick_actions.generate_api_key': 'Generate API Key',
                'dashboard.quick_actions.create_workflow': 'Create Workflow',
                'dashboard.quick_actions.users': 'Users',
                'dashboard.quick_actions.no_quick_create_permission_granted':
                    'No quick create permission granted',
            }
            return translations[key] || key
        },
    }),
}))

const router = createRouter({
    history: createWebHistory(),
    routes: [
        { path: '/dashboard', component: DashboardPage },
        { path: '/entity-definitions', component: { template: '<div>Entity Definitions</div>' } },
        { path: '/entities', component: { template: '<div>Entities</div>' } },
        { path: '/api-keys', component: { template: '<div>API Keys</div>' } },
        { path: '/workflows', component: { template: '<div>Workflows</div>' } },
        { path: '/permissions', component: { template: '<div>Permissions</div>' } },
    ],
})

describe('DashboardPage', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    it('displays entity definitions count', async () => {
        const mockStats: DashboardStats = {
            entity_definitions_count: 5,
            entities: {
                total: 10,
                by_type: [{ entity_type: 'test', count: 10 }],
            },
            workflows: {
                total: 3,
                workflows: [],
            },
            online_users_count: 2,
        }

        const mockAuthStore = {
            canAccessRoute: vi.fn(() => true),
            hasPermission: vi.fn(() => true),
        }
        vi.mocked(useAuthStore).mockReturnValue(mockAuthStore as any)
        vi.mocked(typedHttpClient.getDashboardStats).mockResolvedValue(mockStats)

        const wrapper = mount(DashboardPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        expect(wrapper.text()).toContain('5')
    })

    it('displays entities count with top entity type', async () => {
        const mockStats: DashboardStats = {
            entity_definitions_count: 1,
            entities: {
                total: 15,
                by_type: [
                    { entity_type: 'project', count: 10 },
                    { entity_type: 'task', count: 5 },
                ],
            },
            workflows: {
                total: 0,
                workflows: [],
            },
            online_users_count: 0,
        }

        const mockAuthStore = {
            canAccessRoute: vi.fn(() => true),
            hasPermission: vi.fn(() => true),
        }
        vi.mocked(useAuthStore).mockReturnValue(mockAuthStore as any)
        vi.mocked(typedHttpClient.getDashboardStats).mockResolvedValue(mockStats)

        const wrapper = mount(DashboardPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        expect(wrapper.text()).toContain('15')
        expect(wrapper.text()).toContain('Top: project (10)')
    })

    it('displays workflows count with latest states', async () => {
        const mockStats: DashboardStats = {
            entity_definitions_count: 0,
            entities: {
                total: 0,
                by_type: [],
            },
            workflows: {
                total: 2,
                workflows: [
                    { uuid: '1', name: 'Workflow 1', latest_status: 'finished' },
                    { uuid: '2', name: 'Workflow 2', latest_status: 'failed' },
                ],
            },
            online_users_count: 0,
        }

        const mockAuthStore = {
            canAccessRoute: vi.fn(() => true),
            hasPermission: vi.fn(() => true),
        }
        vi.mocked(useAuthStore).mockReturnValue(mockAuthStore as any)
        vi.mocked(typedHttpClient.getDashboardStats).mockResolvedValue(mockStats)

        const wrapper = mount(DashboardPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        expect(wrapper.text()).toContain('2')
        expect(wrapper.text()).toContain('Latest: Workflow 1: finished, Workflow 2: failed')
    })

    it('displays online users count', async () => {
        const mockStats: DashboardStats = {
            entity_definitions_count: 0,
            entities: {
                total: 0,
                by_type: [],
            },
            workflows: {
                total: 0,
                workflows: [],
            },
            online_users_count: 7,
        }

        const mockAuthStore = {
            canAccessRoute: vi.fn(() => true),
            hasPermission: vi.fn(() => true),
        }
        vi.mocked(useAuthStore).mockReturnValue(mockAuthStore as any)
        vi.mocked(typedHttpClient.getDashboardStats).mockResolvedValue(mockStats)

        const wrapper = mount(DashboardPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        expect(wrapper.text()).toContain('7')
    })

    it('shows loading state initially', () => {
        const mockAuthStore = {
            canAccessRoute: vi.fn(() => true),
            hasPermission: vi.fn(() => true),
        }
        vi.mocked(useAuthStore).mockReturnValue(mockAuthStore as any)
        vi.mocked(typedHttpClient.getDashboardStats).mockImplementation(
            () => new Promise(() => {}) // Never resolves
        )

        const wrapper = mount(DashboardPage, {
            global: {
                plugins: [router],
            },
        })

        // Check for loading indicator (progress circular)
        const progressCircular = wrapper.findComponent({ name: 'VProgressCircular' })
        expect(progressCircular.exists()).toBe(true)
    })

    it('should not load dashboard stats when user lacks permission', async () => {
        const mockAuthStore = {
            canAccessRoute: vi.fn(() => true),
            hasPermission: vi.fn(() => false), // No permission
        }
        vi.mocked(useAuthStore).mockReturnValue(mockAuthStore as any)

        const wrapper = mount(DashboardPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Should not call getDashboardStats
        expect(typedHttpClient.getDashboardStats).not.toHaveBeenCalled()
        // Loading should be false - check via the component's internal state
        // Since loading is a ref, we check that stats haven't been loaded
        const statsText = wrapper.text()
        expect(statsText).toContain('0') // Default values should be 0
    })

    it('should load dashboard stats when user has permission', async () => {
        const mockStats: DashboardStats = {
            entity_definitions_count: 5,
            entities: {
                total: 10,
                by_type: [],
            },
            workflows: {
                total: 3,
                workflows: [],
            },
            online_users_count: 2,
        }

        const mockAuthStore = {
            canAccessRoute: vi.fn(() => true),
            hasPermission: vi.fn(() => true), // Has permission
        }
        vi.mocked(useAuthStore).mockReturnValue(mockAuthStore as any)
        vi.mocked(typedHttpClient.getDashboardStats).mockResolvedValue(mockStats)

        const wrapper = mount(DashboardPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Should call getDashboardStats
        expect(typedHttpClient.getDashboardStats).toHaveBeenCalled()
        // Should have permission check called with DashboardStats
        expect(mockAuthStore.hasPermission).toHaveBeenCalledWith('DashboardStats', 'read')
    })

    it('shows hint message when user has no create permissions', async () => {
        const mockStats: DashboardStats = {
            entity_definitions_count: 5,
            entities: {
                total: 10,
                by_type: [],
            },
            workflows: {
                total: 3,
                workflows: [],
            },
            online_users_count: 2,
        }

        const mockAuthStore = {
            canAccessRoute: vi.fn(() => true),
            hasPermission: vi.fn((namespace: string, permission: string) => {
                // Return true only for DashboardStats:read, false for all create permissions
                if (namespace === 'DashboardStats' && permission === 'read') {
                    return true
                }
                return false
            }),
        }
        vi.mocked(useAuthStore).mockReturnValue(mockAuthStore as any)
        vi.mocked(typedHttpClient.getDashboardStats).mockResolvedValue(mockStats)

        const wrapper = mount(DashboardPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Should show the hint message
        expect(wrapper.text()).toContain('No quick create permission granted')
        // Should not show any quick action buttons (check that buttons are not rendered)
        const html = wrapper.html()
        expect(html).not.toContain('New Entity Definition')
        expect(html).not.toContain('Create Entity')
        expect(html).not.toContain('Generate API Key')
        expect(html).not.toContain('Create Workflow')
    })

    it('does not show hint message when user has create permissions', async () => {
        const mockStats: DashboardStats = {
            entity_definitions_count: 5,
            entities: {
                total: 10,
                by_type: [],
            },
            workflows: {
                total: 3,
                workflows: [],
            },
            online_users_count: 2,
        }

        const mockAuthStore = {
            canAccessRoute: vi.fn(() => true),
            hasPermission: vi.fn((namespace: string, permission: string) => {
                // Return true for DashboardStats:read and Workflows:Create
                if (namespace === 'DashboardStats' && permission === 'read') {
                    return true
                }
                if (namespace === 'Workflows' && permission === 'Create') {
                    return true
                }
                return false
            }),
        }
        vi.mocked(useAuthStore).mockReturnValue(mockAuthStore as any)
        vi.mocked(typedHttpClient.getDashboardStats).mockResolvedValue(mockStats)

        const wrapper = mount(DashboardPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Should not show the hint message
        expect(wrapper.text()).not.toContain('No quick create permission granted')
        // Should show at least one quick action button
        expect(wrapper.text()).toContain('Create Workflow')
    })
})
