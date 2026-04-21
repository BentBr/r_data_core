import { describe, it, expect, vi, beforeEach } from 'vitest'
import { CapabilitiesClient } from './capabilities'
import type { CapabilitiesResponse } from './capabilities'

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

describe('CapabilitiesClient', () => {
    let client: CapabilitiesClient

    beforeEach(() => {
        client = new CapabilitiesClient()
        vi.clearAllMocks()
    })

    describe('getCapabilities', () => {
        it('should call the correct endpoint', async () => {
            const mockCapabilities: CapabilitiesResponse = {
                system_mail_configured: false,
                workflow_mail_configured: false,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockCapabilities,
                }),
            })

            await client.getCapabilities()

            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/system/capabilities'),
                expect.any(Object)
            )
        })

        it('should return capabilities with both false', async () => {
            const mockCapabilities: CapabilitiesResponse = {
                system_mail_configured: false,
                workflow_mail_configured: false,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockCapabilities,
                }),
            })

            const result = await client.getCapabilities()

            expect(result.system_mail_configured).toBe(false)
            expect(result.workflow_mail_configured).toBe(false)
        })

        it('should return capabilities with system_mail_configured true', async () => {
            const mockCapabilities: CapabilitiesResponse = {
                system_mail_configured: true,
                workflow_mail_configured: false,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockCapabilities,
                }),
            })

            const result = await client.getCapabilities()

            expect(result.system_mail_configured).toBe(true)
            expect(result.workflow_mail_configured).toBe(false)
        })

        it('should return capabilities with workflow_mail_configured true', async () => {
            const mockCapabilities: CapabilitiesResponse = {
                system_mail_configured: false,
                workflow_mail_configured: true,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockCapabilities,
                }),
            })

            const result = await client.getCapabilities()

            expect(result.system_mail_configured).toBe(false)
            expect(result.workflow_mail_configured).toBe(true)
        })

        it('should return capabilities with both true', async () => {
            const mockCapabilities: CapabilitiesResponse = {
                system_mail_configured: true,
                workflow_mail_configured: true,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockCapabilities,
                }),
            })

            const result = await client.getCapabilities()

            expect(result.system_mail_configured).toBe(true)
            expect(result.workflow_mail_configured).toBe(true)
        })

        it('should throw on HTTP error response', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 500,
                statusText: 'Internal Server Error',
                json: async () => ({
                    status: 'Error',
                    message: 'Server error',
                }),
            })

            await expect(client.getCapabilities()).rejects.toThrow()
        })

        it('should use GET method by default', async () => {
            const mockCapabilities: CapabilitiesResponse = {
                system_mail_configured: false,
                workflow_mail_configured: false,
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: mockCapabilities,
                }),
            })

            await client.getCapabilities()

            expect(mockFetch).toHaveBeenCalledWith(
                expect.any(String),
                expect.objectContaining({
                    headers: expect.objectContaining({
                        Authorization: 'Bearer test-token',
                    }),
                })
            )
        })
    })
})
