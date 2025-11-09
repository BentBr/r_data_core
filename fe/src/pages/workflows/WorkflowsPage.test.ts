import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import WorkflowsPage from './WorkflowsPage.vue'

const mockGetWorkflows = vi.fn()
const mockRunWorkflow = vi.fn().mockResolvedValue({})
const mockUploadRunFile = vi.fn().mockResolvedValue({ run_uuid: 'r1', staged_items: 3 })

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getWorkflows: (...args: any[]) => mockGetWorkflows(...args),
        runWorkflow: (...args: any[]) => mockRunWorkflow(...args),
        uploadRunFile: (...args: any[]) => mockUploadRunFile(...args),
        getWorkflowRuns: vi.fn().mockResolvedValue({ data: [], meta: { pagination: { total: 0 } } }),
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
    })

    it.skip('loads workflows and runs "run now" without upload', async () => {
        const wrapper = mount(WorkflowsPage)
        // Wait for initial loadWorkflows
        await vi.waitUntil(() => mockGetWorkflows.mock.calls.length > 0, { timeout: 1000 })

        // Click the run-now icon button using title attribute set from translations ('run_now')
        const runBtn = wrapper.find('button[title="run_now"]')
        expect(runBtn).toBeTruthy()
        await runBtn.trigger('click')

        // Confirm run (non-upload path)
        const confirm = wrapper.findAll('button').find(b => /run_button/i.test(b.text()))
        expect(confirm).toBeTruthy()
        await confirm!.trigger('click')
        expect(mockRunWorkflow).toHaveBeenCalledTimes(1)
        expect(showSuccess).toHaveBeenCalled()
    })

    it.skip('history tab includes "all" option', async () => {
        const wrapper = mount(WorkflowsPage)
        await vi.waitUntil(() => mockGetWorkflows.mock.calls.length > 0, { timeout: 1000 })

        // Switch to history tab by clicking it
        const tabBtns = wrapper.findAll('[role="tab"]')
        const historyTab = tabBtns.find(t => /history/i.test(t.text()))
        expect(historyTab).toBeTruthy()
        await historyTab!.trigger('click')

        // The select should include an option labeled 'all' (from translations)
        expect(wrapper.text().toLowerCase()).toContain('all')
    })

    it.skip('disables run button when upload enabled but no file selected', async () => {
        const wrapper = mount(WorkflowsPage)
        await vi.waitUntil(() => mockGetWorkflows.mock.calls.length > 0, { timeout: 1000 })

        // Open run dialog
        const runBtn = wrapper.find('button[title="run_now"]')
        expect(runBtn).toBeTruthy()
        await runBtn.trigger('click')

        // Toggle upload switch
        const switchInput = wrapper.find('input[type="checkbox"]')
        expect(switchInput.exists()).toBe(true)
        await switchInput.setValue(true)

        // Run button should be disabled without a file
        const runAction = wrapper.findAll('button').find(b => /run_button/i.test(b.text()))
        expect(runAction).toBeTruthy()
        expect(runAction!.attributes('disabled')).toBeDefined()
    })
})


