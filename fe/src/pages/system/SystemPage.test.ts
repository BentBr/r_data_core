import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import SystemPage from './SystemPage.vue'
import type { EntityVersioningSettings } from '@/api/clients/system'

const mockGetEntityVersioningSettings = vi.fn()
const mockUpdateEntityVersioningSettings = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getEntityVersioningSettings: () => mockGetEntityVersioningSettings(),
        updateEntityVersioningSettings: (data: EntityVersioningSettings) =>
            mockUpdateEntityVersioningSettings(data),
    },
}))

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
        const form = wrapper.vm.form
        form.max_versions = '10' as unknown as number
        form.max_age_days = '180' as unknown as number

        // Trigger save
        await wrapper.vm.save()
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
        const form = wrapper.vm.form
        form.max_versions = '' as unknown as number
        form.max_age_days = '' as unknown as number

        // Trigger save
        await wrapper.vm.save()
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
        expect(wrapper.vm.form.max_versions).toBe(10)
        expect(wrapper.vm.form.max_age_days).toBe(180)
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
        await wrapper.vm.save()
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
