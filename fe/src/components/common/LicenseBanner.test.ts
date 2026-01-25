import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import LicenseBanner from './LicenseBanner.vue'
import { useLicenseStore } from '@/stores/license'
import { typedHttpClient } from '@/api/typed-client'
import type { LicenseStatus } from '@/api/clients/system'

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getLicenseStatus: vi.fn(),
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (key: string) => key.split('.').pop() || key,
    }),
}))

describe('LicenseBanner', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.clearAllMocks()
        localStorage.clear()
    })

    it('should not show banner when license is valid', async () => {
        const mockStatus: LicenseStatus = {
            state: 'valid',
            company: 'Test Company',
            license_type: 'Enterprise',
            license_id: 'test-id',
            issued_at: '2024-01-01T00:00:00Z',
            version: 'v1',
            verified_at: '2024-01-02T00:00:00Z',
            error_message: null,
        }

        vi.mocked(typedHttpClient.getLicenseStatus).mockResolvedValue(mockStatus)

        const store = useLicenseStore()
        await store.loadLicenseStatus()

        const wrapper = mount(LicenseBanner)
        expect(wrapper.find('.dismissable-banner').exists()).toBe(false)
    })

    it('should show banner when license state is none', async () => {
        const mockStatus: LicenseStatus = {
            state: 'none',
            company: null,
            license_type: null,
            license_id: null,
            issued_at: null,
            version: null,
            verified_at: '2024-01-02T00:00:00Z',
            error_message: null,
        }

        vi.mocked(typedHttpClient.getLicenseStatus).mockResolvedValue(mockStatus)

        const store = useLicenseStore()
        await store.loadLicenseStatus()

        const wrapper = mount(LicenseBanner)
        expect(wrapper.find('.dismissable-banner').exists()).toBe(true)
        // Translation mock returns last part of key: 'license.banner.no_license' -> 'no_license'
        expect(wrapper.text()).toContain('no_license')
    })

    it('should show banner when license state is invalid', async () => {
        const mockStatus: LicenseStatus = {
            state: 'invalid',
            company: 'Test Company',
            license_type: 'Enterprise',
            license_id: 'test-id',
            issued_at: '2024-01-01T00:00:00Z',
            version: 'v1',
            verified_at: '2024-01-02T00:00:00Z',
            error_message: 'Invalid license key',
        }

        vi.mocked(typedHttpClient.getLicenseStatus).mockResolvedValue(mockStatus)

        const store = useLicenseStore()
        await store.loadLicenseStatus()

        const wrapper = mount(LicenseBanner)
        expect(wrapper.find('.dismissable-banner').exists()).toBe(true)
        // Translation mock returns last part of key: 'license.banner.invalid_license' -> 'invalid_license'
        expect(wrapper.text()).toContain('invalid_license')
    })

    it('should show banner when license state is error', async () => {
        const mockStatus: LicenseStatus = {
            state: 'error',
            company: 'Test Company',
            license_type: 'Enterprise',
            license_id: 'test-id',
            issued_at: '2024-01-01T00:00:00Z',
            version: 'v1',
            verified_at: '2024-01-02T00:00:00Z',
            error_message: 'Network error',
        }

        vi.mocked(typedHttpClient.getLicenseStatus).mockResolvedValue(mockStatus)

        const store = useLicenseStore()
        await store.loadLicenseStatus()

        const wrapper = mount(LicenseBanner)
        expect(wrapper.find('.dismissable-banner').exists()).toBe(true)
        // Translation mock returns last part of key: 'license.banner.error_license' -> 'error_license'
        expect(wrapper.text()).toContain('error_license')
    })

    it('should not show banner when dismissed', async () => {
        const mockStatus: LicenseStatus = {
            state: 'none',
            company: null,
            license_type: null,
            license_id: null,
            issued_at: null,
            version: null,
            verified_at: '2024-01-02T00:00:00Z',
            error_message: null,
        }

        vi.mocked(typedHttpClient.getLicenseStatus).mockResolvedValue(mockStatus)

        const store = useLicenseStore()
        await store.loadLicenseStatus()
        store.dismissLicenseBanner()

        const wrapper = mount(LicenseBanner)
        expect(wrapper.find('.dismissable-banner').exists()).toBe(false)
    })

    it('should call dismissLicenseBanner when dismiss button is clicked', async () => {
        const mockStatus: LicenseStatus = {
            state: 'none',
            company: null,
            license_type: null,
            license_id: null,
            issued_at: null,
            version: null,
            verified_at: '2024-01-02T00:00:00Z',
            error_message: null,
        }

        vi.mocked(typedHttpClient.getLicenseStatus).mockResolvedValue(mockStatus)

        const store = useLicenseStore()
        await store.loadLicenseStatus()

        const wrapper = mount(LicenseBanner)
        const dismissSpy = vi.spyOn(store, 'dismissLicenseBanner')

        const dismissButton = wrapper.find('button')
        await dismissButton.trigger('click')

        expect(dismissSpy).toHaveBeenCalled()
    })
})
