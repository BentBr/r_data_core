import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import LicenseBanner from './LicenseBanner.vue'
import { useLicenseStore } from '@/stores/license'

describe('LicenseBanner', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
    })

    it('should not show banner when license is valid', () => {
        const store = useLicenseStore()
        store.licenseStatus = {
            state: 'valid',
            company: 'Test Company',
            license_type: 'Enterprise',
            license_id: 'test-id',
            issued_at: '2024-01-01T00:00:00Z',
            version: 'v1',
            verified_at: '2024-01-02T00:00:00Z',
            error_message: null,
        }

        const wrapper = mount(LicenseBanner)
        expect(wrapper.find('.dismissable-banner').exists()).toBe(false)
    })

    it('should show banner when license state is none', () => {
        const store = useLicenseStore()
        // @ts-expect-error - accessing private state for testing
        store.licenseStatus.value = {
            state: 'none',
            company: null,
            license_type: null,
            license_id: null,
            issued_at: null,
            version: null,
            verified_at: '2024-01-02T00:00:00Z',
            error_message: null,
        }

        const wrapper = mount(LicenseBanner)
        expect(wrapper.find('.dismissable-banner').exists()).toBe(true)
        expect(wrapper.text()).toContain('No license key configured')
    })

    it('should show banner when license state is invalid', () => {
        const store = useLicenseStore()
        // @ts-expect-error - accessing private state for testing
        store.licenseStatus.value = {
            state: 'invalid',
            company: 'Test Company',
            license_type: 'Enterprise',
            license_id: 'test-id',
            issued_at: '2024-01-01T00:00:00Z',
            version: 'v1',
            verified_at: '2024-01-02T00:00:00Z',
            error_message: 'Invalid license key',
        }

        const wrapper = mount(LicenseBanner)
        expect(wrapper.find('.dismissable-banner').exists()).toBe(true)
        expect(wrapper.text()).toContain('License key is invalid')
    })

    it('should show banner when license state is error', () => {
        const store = useLicenseStore()
        // @ts-expect-error - accessing private state for testing
        store.licenseStatus.value = {
            state: 'error',
            company: 'Test Company',
            license_type: 'Enterprise',
            license_id: 'test-id',
            issued_at: '2024-01-01T00:00:00Z',
            version: 'v1',
            verified_at: '2024-01-02T00:00:00Z',
            error_message: 'Network error',
        }

        const wrapper = mount(LicenseBanner)
        expect(wrapper.find('.dismissable-banner').exists()).toBe(true)
        expect(wrapper.text()).toContain('License verification failed')
    })

    it('should not show banner when dismissed', () => {
        const store = useLicenseStore()
        // @ts-expect-error - accessing private state for testing
        store.licenseStatus.value = {
            state: 'none',
            company: null,
            license_type: null,
            license_id: null,
            issued_at: null,
            version: null,
            verified_at: '2024-01-02T00:00:00Z',
            error_message: null,
        }
        store.dismissLicenseBanner()

        const wrapper = mount(LicenseBanner)
        expect(wrapper.find('.dismissable-banner').exists()).toBe(false)
    })

    it('should call dismissLicenseBanner when dismiss button is clicked', async () => {
        const store = useLicenseStore()
        // @ts-expect-error - accessing private state for testing
        store.licenseStatus.value = {
            state: 'none',
            company: null,
            license_type: null,
            license_id: null,
            issued_at: null,
            version: null,
            verified_at: '2024-01-02T00:00:00Z',
            error_message: null,
        }

        const wrapper = mount(LicenseBanner)
        const dismissSpy = vi.spyOn(store, 'dismissLicenseBanner')

        const dismissButton = wrapper.find('button')
        await dismissButton.trigger('click')

        expect(dismissSpy).toHaveBeenCalled()
    })
})
