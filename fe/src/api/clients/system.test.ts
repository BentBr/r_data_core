import { describe, it, expect, vi, beforeEach } from 'vitest'
import { SystemClient } from './system'
import type {
    EntityVersioningSettings,
    WorkflowRunLogSettings,
    LicenseStatus,
    SystemVersions,
} from './system'

// Mock fetch and dependencies
const mockFetch = vi.fn()
global.fetch = mockFetch

vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        token: 'test-token',
        refreshTokens: vi.fn(),
        logout: vi.fn(),
    }),
}))

vi.mock('@/utils/cookies', () => ({
    getRefreshToken: () => 'refresh-token',
}))

vi.mock('@/env-check', () => ({
    env: {
        apiBaseUrl: 'http://localhost:3000',
        enableApiLogging: false,
        devMode: false,
        defaultPageSize: 10,
    },
    buildApiUrl: (endpoint: string) => `http://localhost:3000${endpoint}`,
}))

describe('SystemClient', () => {
    let client: SystemClient

    beforeEach(() => {
        client = new SystemClient()
        vi.clearAllMocks()
    })

    describe('getEntityVersioningSettings', () => {
        it('should get entity versioning settings', async () => {
            const mockSettings: EntityVersioningSettings = {
                enabled: true,
                max_versions: 10,
                max_age_days: 90,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockSettings,
                }),
            })

            const result = await client.getEntityVersioningSettings()

            expect(result).toBeDefined()
            expect(result.enabled).toBe(true)
            expect(result.max_versions).toBe(10)
            expect(result.max_age_days).toBe(90)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/system/settings/entity-versioning'),
                expect.any(Object)
            )
        })

        it('should handle settings with null optional fields', async () => {
            const mockSettings: EntityVersioningSettings = {
                enabled: false,
                max_versions: null,
                max_age_days: null,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockSettings,
                }),
            })

            const result = await client.getEntityVersioningSettings()

            expect(result.enabled).toBe(false)
            expect(result.max_versions).toBeNull()
            expect(result.max_age_days).toBeNull()
        })
    })

    describe('updateEntityVersioningSettings', () => {
        it('should update entity versioning settings', async () => {
            const payload: EntityVersioningSettings = {
                enabled: true,
                max_versions: 20,
                max_age_days: 180,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'Updated',
                    data: payload,
                }),
            })

            const result = await client.updateEntityVersioningSettings(payload)

            expect(result).toBeDefined()
            expect(result.enabled).toBe(true)
            expect(result.max_versions).toBe(20)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/system/settings/entity-versioning'),
                expect.objectContaining({
                    method: 'PUT',
                    body: JSON.stringify(payload),
                })
            )
        })

        it('should throw on error response when updating', async () => {
            const payload: EntityVersioningSettings = {
                enabled: true,
                max_versions: -1,
            }

            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 422,
                json: async () => ({ status: 'Error', message: 'Validation failed' }),
            })

            await expect(client.updateEntityVersioningSettings(payload)).rejects.toThrow()
        })
    })

    describe('getWorkflowRunLogSettings', () => {
        it('should get workflow run log settings', async () => {
            const mockSettings: WorkflowRunLogSettings = {
                enabled: true,
                max_runs: 100,
                max_age_days: 30,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockSettings,
                }),
            })

            const result = await client.getWorkflowRunLogSettings()

            expect(result).toBeDefined()
            expect(result.enabled).toBe(true)
            expect(result.max_runs).toBe(100)
            expect(result.max_age_days).toBe(30)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/system/settings/workflow-run-logs'),
                expect.any(Object)
            )
        })

        it('should handle settings with null optional fields', async () => {
            const mockSettings: WorkflowRunLogSettings = {
                enabled: false,
                max_runs: null,
                max_age_days: null,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockSettings,
                }),
            })

            const result = await client.getWorkflowRunLogSettings()

            expect(result.enabled).toBe(false)
            expect(result.max_runs).toBeNull()
            expect(result.max_age_days).toBeNull()
        })
    })

    describe('updateWorkflowRunLogSettings', () => {
        it('should update workflow run log settings', async () => {
            const payload: WorkflowRunLogSettings = {
                enabled: true,
                max_runs: 500,
                max_age_days: 60,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'Updated',
                    data: payload,
                }),
            })

            const result = await client.updateWorkflowRunLogSettings(payload)

            expect(result).toBeDefined()
            expect(result.enabled).toBe(true)
            expect(result.max_runs).toBe(500)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/system/settings/workflow-run-logs'),
                expect.objectContaining({
                    method: 'PUT',
                    body: JSON.stringify(payload),
                })
            )
        })

        it('should throw on error response when updating', async () => {
            const payload: WorkflowRunLogSettings = {
                enabled: true,
                max_runs: -5,
            }

            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 422,
                json: async () => ({ status: 'Error', message: 'Validation failed' }),
            })

            await expect(client.updateWorkflowRunLogSettings(payload)).rejects.toThrow()
        })
    })

    describe('getLicenseStatus', () => {
        it('should get a valid license status', async () => {
            const mockStatus: LicenseStatus = {
                state: 'valid',
                company: 'Acme Corp',
                license_type: 'enterprise',
                license_id: 'LIC-001-2024',
                issued_at: '2024-01-01T00:00:00Z',
                expires_at: '2025-01-01T00:00:00Z',
                version: '1.0',
                verified_at: '2024-06-15T12:00:00Z',
                error_message: null,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockStatus,
                }),
            })

            const result = await client.getLicenseStatus()

            expect(result).toBeDefined()
            expect(result.state).toBe('valid')
            expect(result.company).toBe('Acme Corp')
            expect(result.verified_at).toBe('2024-06-15T12:00:00Z')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/system/license'),
                expect.any(Object)
            )
        })

        it('should handle invalid license status', async () => {
            const mockStatus: LicenseStatus = {
                state: 'invalid',
                verified_at: '2024-06-15T12:00:00Z',
                error_message: 'License signature mismatch',
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockStatus,
                }),
            })

            const result = await client.getLicenseStatus()

            expect(result.state).toBe('invalid')
            expect(result.error_message).toBe('License signature mismatch')
        })

        it('should handle missing license (none state)', async () => {
            const mockStatus: LicenseStatus = {
                state: 'none',
                verified_at: '2024-06-15T12:00:00Z',
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockStatus,
                }),
            })

            const result = await client.getLicenseStatus()

            expect(result.state).toBe('none')
            expect(result.company).toBeUndefined()
        })
    })

    describe('getSystemVersions', () => {
        it('should get system versions with all components', async () => {
            const mockVersions: SystemVersions = {
                core: '0.4.8',
                worker: {
                    name: 'worker',
                    version: '0.4.8',
                    last_seen_at: '2024-06-15T12:00:00Z',
                },
                maintenance: {
                    name: 'maintenance',
                    version: '0.4.8',
                    last_seen_at: '2024-06-15T11:55:00Z',
                },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockVersions,
                }),
            })

            const result = await client.getSystemVersions()

            expect(result).toBeDefined()
            expect(result.core).toBe('0.4.8')
            expect(result.worker?.version).toBe('0.4.8')
            expect(result.maintenance?.version).toBe('0.4.8')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/system/versions'),
                expect.any(Object)
            )
        })

        it('should handle system versions when optional components are absent', async () => {
            const mockVersions: SystemVersions = {
                core: '0.4.8',
                worker: null,
                maintenance: null,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockVersions,
                }),
            })

            const result = await client.getSystemVersions()

            expect(result.core).toBe('0.4.8')
            expect(result.worker).toBeNull()
            expect(result.maintenance).toBeNull()
        })
    })
})
