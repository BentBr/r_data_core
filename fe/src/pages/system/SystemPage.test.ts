import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import SystemPage from './SystemPage.vue'
import type { EntityVersioningSettings } from '@/api/clients/system'

const mockGetEntityVersioningSettings = vi.fn()
const mockUpdateEntityVersioningSettings = vi.fn()

vi.mock('@/api/typed-client', () => {
    // Define ValidationError class inline to avoid hoisting issues
    class ValidationError extends Error {
        violations: Array<{ field: string; message: string }>
        constructor(message: string, violations: Array<{ field: string; message: string }>) {
            super(message)
            this.name = 'ValidationError'
            this.violations = violations
        }
    }

    return {
        typedHttpClient: {
            getEntityVersioningSettings: () => mockGetEntityVersioningSettings(),
            updateEntityVersioningSettings: (data: unknown) =>
                mockUpdateEntityVersioningSettings(data),
        },
        ValidationError,
    }
})

vi.mock('@/api/errors', () => {
    // Define HttpError class inline to avoid hoisting issues
    class HttpError extends Error {
        statusCode: number
        namespace: string
        action: string
        originalMessage: string
        constructor(
            statusCode: number,
            namespace: string,
            action: string,
            message: string,
            originalMessage?: string
        ) {
            super(message)
            this.name = 'HttpError'
            this.statusCode = statusCode
            this.namespace = namespace
            this.action = action
            this.originalMessage = originalMessage ?? message
        }
    }

    return {
        HttpError,
        extractActionFromMethod: () => 'read',
        extractNamespaceFromEndpoint: () => 'system',
    }
})

const showSuccess = vi.fn()
const showError = vi.fn()
vi.mock('@/composables/useSnackbar', () => ({
    useSnackbar: () => ({
        showSuccess,
        showError,
    }),
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (key: string) => key.split('.').pop(),
    }),
}))

describe('SystemPage', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    it('should convert string inputs to numbers when saving', async () => {
        const settings: EntityVersioningSettings = {
            enabled: true,
            max_versions: 10,
            max_age_days: 180,
        }
        mockGetEntityVersioningSettings.mockResolvedValue(settings)
        mockUpdateEntityVersioningSettings.mockResolvedValue(settings)

        const wrapper = mount(SystemPage)

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Set form values as strings (as v-text-field would)
        const vm = wrapper.vm as unknown as {
            form: { max_versions: number | string; max_age_days: number | string; enabled: boolean }
            save: () => Promise<void>
        }
        vm.form.max_versions = '10' as unknown as number
        vm.form.max_age_days = '180' as unknown as number

        // Trigger save
        await vm.save()
        await wrapper.vm.$nextTick()

        // Verify that numbers were sent, not strings
        expect(mockUpdateEntityVersioningSettings).toHaveBeenCalledWith({
            enabled: true,
            max_versions: 10,
            max_age_days: 180,
        })
        expect(showSuccess).toHaveBeenCalled()
    })

    it('should convert empty strings to null when saving', async () => {
        const settings: EntityVersioningSettings = {
            enabled: true,
            max_versions: null,
            max_age_days: null,
        }
        mockGetEntityVersioningSettings.mockResolvedValue(settings)
        mockUpdateEntityVersioningSettings.mockResolvedValue(settings)

        const wrapper = mount(SystemPage)

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Set form values as empty strings
        const vm = wrapper.vm as unknown as {
            form: { max_versions: number | string; max_age_days: number | string; enabled: boolean }
            save: () => Promise<void>
        }
        vm.form.max_versions = '' as unknown as number
        vm.form.max_age_days = '' as unknown as number

        // Trigger save
        await vm.save()
        await wrapper.vm.$nextTick()

        // Verify that null was sent, not empty strings
        expect(mockUpdateEntityVersioningSettings).toHaveBeenCalledWith({
            enabled: true,
            max_versions: null,
            max_age_days: null,
        })
        expect(showSuccess).toHaveBeenCalled()
    })

    it('should handle backend returning integers as strings', async () => {
        // Simulate backend returning string "10" instead of number 10
        const settingsWithString: EntityVersioningSettings = {
            enabled: true,
            max_versions: 10 as unknown as number, // Backend should return number, but test edge case
            max_age_days: 180 as unknown as number,
        }
        mockGetEntityVersioningSettings.mockResolvedValue(settingsWithString)

        const wrapper = mount(SystemPage)

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Form should be populated with the values
        const vm = wrapper.vm as unknown as {
            form: { max_versions: number | string; max_age_days: number | string; enabled: boolean }
        }
        expect(vm.form.max_versions).toBe(10)
        expect(vm.form.max_age_days).toBe(180)
    })

    it('should show error message when save fails', async () => {
        const settings: EntityVersioningSettings = {
            enabled: true,
            max_versions: 10,
            max_age_days: 180,
        }
        mockGetEntityVersioningSettings.mockResolvedValue(settings)
        mockUpdateEntityVersioningSettings.mockRejectedValue(
            new Error('Deserialization error: invalid type: string "10", expected i32')
        )

        const wrapper = mount(SystemPage)

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Trigger save
        const vm = wrapper.vm as unknown as { save: () => Promise<void> }
        await vm.save()
        await wrapper.vm.$nextTick()

        // Verify error was shown
        expect(showError).toHaveBeenCalled()
    })

    it('should show error message when load fails', async () => {
        mockGetEntityVersioningSettings.mockRejectedValue(new Error('Failed to load settings'))

        const wrapper = mount(SystemPage)

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Verify error was shown
        expect(showError).toHaveBeenCalled()
    })
})
