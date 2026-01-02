import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useLicenseStore } from './license'
import { typedHttpClient } from '@/api/typed-client'
import type { LicenseStatus } from '@/api/clients/system'

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getLicenseStatus: vi.fn(),
    },
}))

describe('LicenseStore', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.clearAllMocks()
        localStorage.clear()
    })

    it('should initialize with null license status', () => {
        const store = useLicenseStore()
        expect(store.licenseStatus).toBeNull()
        expect(store.shouldShowBanner).toBe(false)
    })

    it('should load license status', async () => {
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

        expect(store.licenseStatus).toEqual(mockStatus)
        expect(store.isLicenseValid).toBe(true)
        expect(store.shouldShowBanner).toBe(false)
    })

    it('should show banner for none state', async () => {
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

        expect(store.shouldShowBanner).toBe(true)
    })

    it('should show banner for invalid state', async () => {
        const mockStatus: LicenseStatus = {
            state: 'invalid',
            company: 'Test Company',
            license_type: 'Enterprise',
            license_id: 'test-id',
            issued_at: '2024-01-01T00:00:00Z',
            version: 'v1',
            verified_at: '2024-01-02T00:00:00Z',
            error_message: 'Invalid license',
        }

        vi.mocked(typedHttpClient.getLicenseStatus).mockResolvedValue(mockStatus)

        const store = useLicenseStore()
        await store.loadLicenseStatus()

        expect(store.shouldShowBanner).toBe(true)
    })

    it('should show banner for error state', async () => {
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

        expect(store.shouldShowBanner).toBe(true)
    })

    it('should not show banner for valid state', async () => {
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

        expect(store.shouldShowBanner).toBe(false)
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

        expect(store.shouldShowBanner).toBe(false)
        expect(localStorage.getItem('license_banner_dismissed')).toBe('true')
    })

    it('should reset banner dismissal', () => {
        const store = useLicenseStore()
        store.dismissLicenseBanner()
        expect(localStorage.getItem('license_banner_dismissed')).toBe('true')

        store.resetBannerDismissal()
        expect(localStorage.getItem('license_banner_dismissed')).toBeNull()
        expect(store.shouldShowBanner).toBe(false) // Still false because no license status
    })

    it('should handle load error gracefully', async () => {
        vi.mocked(typedHttpClient.getLicenseStatus).mockRejectedValue(new Error('Network error'))

        const store = useLicenseStore()
        await store.loadLicenseStatus()

        expect(store.error).toBeTruthy()
        expect(store.isLoading).toBe(false)
    })
})
