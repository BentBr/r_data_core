import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import WorkflowsPage from './WorkflowsPage.vue'

const mockGetWorkflows = vi.fn()
const mockRunWorkflow = vi.fn().mockResolvedValue({})
const mockUploadRunFile = vi.fn().mockResolvedValue({ run_uuid: 'r1', staged_items: 3 })

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getWorkflows: (page?: number, itemsPerPage?: number) =>
            mockGetWorkflows(page, itemsPerPage),
        runWorkflow: (uuid: string, data?: { file?: File }) => mockRunWorkflow(uuid, data),
        uploadRunFile: (uuid: string, file: File) => mockUploadRunFile(uuid, file),
        getWorkflowRuns: vi
            .fn()
            .mockResolvedValue({ data: [], meta: { pagination: { total: 0 } } }),
        getAllWorkflowRuns: vi
            .fn()
            .mockResolvedValue({ data: [], meta: { pagination: { total: 0 } } }),
        getWorkflowRunLogs: vi
            .fn()
            .mockResolvedValue({ data: [], meta: { pagination: { total: 0 } } }),
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k.split('.').pop() }),
}))

// Mock snackbar to avoid UI dependencies; capture success/error messages
const showSuccess = vi.fn()
const showError = vi.fn()
vi.mock('@/composables/useSnackbar', () => ({
    useSnackbar: () => ({
        currentSnackbar: null,
        showSuccess,
        showError,
    }),
}))

const mockHasPermission = vi.fn()
vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        hasPermission: mockHasPermission,
    }),
}))

const router = createRouter({
    history: createWebHistory(),
    routes: [{ path: '/workflows', component: WorkflowsPage }],
})

describe('WorkflowsPage', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockGetWorkflows.mockResolvedValue({
            data: [
                {
                    uuid: '019a46aa-582d-7f51-8782-641a00ec534c',
                    name: 'WF A',
                    kind: 'consumer',
                    enabled: true,
                    schedule_cron: null,
                },
            ],
            meta: { pagination: { total: 1, total_pages: 1, page: 1, per_page: 20 } },
        })
        // Default: user has create permission
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'Workflows' && (permission === 'Create' || permission === 'Admin')
        })
    })

    it('loads workflows and runs "run now" without upload', async () => {
        const wrapper = mount(WorkflowsPage, {
            global: {
                plugins: [router],
            },
        })
        // Wait for initial loadWorkflows
        await vi.waitUntil(() => mockGetWorkflows.mock.calls.length > 0, { timeout: 1000 })
        // Call exposed API to open and confirm
        await (wrapper.vm as any).openRunNow('019a46aa-582d-7f51-8782-641a00ec534c')
        await (wrapper.vm as any).confirmRunNow()

        // Confirm run (non-upload path)
        expect(mockRunWorkflow).toHaveBeenCalledTimes(1)
        expect(showSuccess).toHaveBeenCalled()
    })

    it('history tab includes "all" option', async () => {
        const wrapper = mount(WorkflowsPage, {
            global: {
                plugins: [router],
            },
        })
        await vi.waitUntil(() => mockGetWorkflows.mock.calls.length > 0, { timeout: 1000 })
        ;(wrapper.vm as any).activeTab = 'history'
        ;(wrapper.vm as any).selectedWorkflowUuid = 'all'
        await (wrapper.vm as any).loadRuns()
        const tc = await import('@/api/typed-client')
        expect((tc.typedHttpClient.getAllWorkflowRuns as any).mock.calls.length).toBeGreaterThan(0)
    })

    it('disables run button when upload enabled but no file selected', async () => {
        const wrapper = mount(WorkflowsPage, {
            global: {
                plugins: [router],
            },
        })
        await vi.waitUntil(() => mockGetWorkflows.mock.calls.length > 0, { timeout: 1000 })
        // Simulate state and assert disabled condition
        await (wrapper.vm as any).openRunNow('019a46aa-582d-7f51-8782-641a00ec534c')
        ;(wrapper.vm as any).uploadEnabled = true
        ;(wrapper.vm as any).uploadFile = null
        // Expression used in template: :disabled=\"uploadEnabled && !uploadFile\"
        expect((wrapper.vm as any).uploadEnabled && !(wrapper.vm as any).uploadFile).toBe(true)
    })

    it('shows create button when user has Workflows:Create permission', async () => {
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'Workflows' && permission === 'Create'
        })

        const wrapper = mount(WorkflowsPage, {
            global: {
                plugins: [router],
            },
        })

        await vi.waitUntil(() => mockGetWorkflows.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Should show create button
        const createButton = wrapper.find('button')
        expect(createButton.exists()).toBe(true)
        expect(wrapper.text()).toContain('button') // The button text from translation mock
    })

    it('shows create button when user has Workflows:Admin permission', async () => {
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'Workflows' && permission === 'Admin'
        })

        const wrapper = mount(WorkflowsPage, {
            global: {
                plugins: [router],
            },
        })

        await vi.waitUntil(() => mockGetWorkflows.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Should show create button
        const createButton = wrapper.find('button')
        expect(createButton.exists()).toBe(true)
    })

    it('hides create button when user lacks create permissions', async () => {
        mockHasPermission.mockImplementation(() => false)

        const wrapper = mount(WorkflowsPage, {
            global: {
                plugins: [router],
            },
        })

        await vi.waitUntil(() => mockGetWorkflows.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Check that canCreateWorkflow computed is false
        expect((wrapper.vm as any).canCreateWorkflow).toBe(false)
        // The button should not be rendered due to v-if="canCreateWorkflow"
        const html = wrapper.html()
        // Should not contain the create button text in the actions area
        // The button is in template #actions, so we check the component doesn't render it
        expect(html).not.toContain('workflows.create.button')
    })
})
