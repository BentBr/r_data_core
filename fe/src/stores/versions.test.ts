import { describe, it, expect, beforeEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useVersionStore } from './versions'
import { typedHttpClient } from '@/api/typed-client'
import type { SystemVersions } from '@/api/clients/system'

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getSystemVersions: vi.fn(),
    },
}))

vi.mock('@/env-check', () => ({
    env: {
        enableApiLogging: false,
    },
}))

describe('VersionStore', () => {
    beforeEach(() => {
        setActivePinia(createPinia())
        vi.clearAllMocks()
    })

    it('should initialize with null versions', () => {
        const store = useVersionStore()
        expect(store.coreVersion).toBeNull()
        expect(store.workerVersion).toBeNull()
        expect(store.maintenanceVersion).toBeNull()
        expect(store.hasVersions).toBe(false)
    })

    it('should have FE version from build constant', () => {
        const store = useVersionStore()
        // FE version is injected at build time, should be defined
        expect(store.feVersion).toBeDefined()
    })

    it('should load all system versions', async () => {
        const mockVersions: SystemVersions = {
            core: '1.0.0',
            worker: {
                name: 'worker',
                version: '1.0.0',
                last_seen_at: '2024-01-01T00:00:00Z',
            },
            maintenance: {
                name: 'maintenance',
                version: '1.0.0',
                last_seen_at: '2024-01-01T00:00:00Z',
            },
        }

        vi.mocked(typedHttpClient.getSystemVersions).mockResolvedValue(mockVersions)

        const store = useVersionStore()
        await store.loadVersions()

        expect(store.coreVersion).toBe('1.0.0')
        expect(store.workerVersion).toEqual(mockVersions.worker)
        expect(store.maintenanceVersion).toEqual(mockVersions.maintenance)
        expect(store.hasVersions).toBe(true)
        expect(store.lastFetchedAt).not.toBeNull()
    })

    it('should load versions with only core', async () => {
        const mockVersions: SystemVersions = {
            core: '2.0.0',
            worker: undefined,
            maintenance: undefined,
        }

        vi.mocked(typedHttpClient.getSystemVersions).mockResolvedValue(mockVersions)

        const store = useVersionStore()
        await store.loadVersions()

        expect(store.coreVersion).toBe('2.0.0')
        expect(store.workerVersion).toBeNull()
        expect(store.maintenanceVersion).toBeNull()
        expect(store.hasVersions).toBe(true)
    })

    it('should provide all versions in allVersions getter', async () => {
        const mockVersions: SystemVersions = {
            core: '1.0.0',
            worker: {
                name: 'worker',
                version: '1.1.0',
                last_seen_at: '2024-01-01T00:00:00Z',
            },
            maintenance: {
                name: 'maintenance',
                version: '1.2.0',
                last_seen_at: '2024-01-01T00:00:00Z',
            },
        }

        vi.mocked(typedHttpClient.getSystemVersions).mockResolvedValue(mockVersions)

        const store = useVersionStore()
        await store.loadVersions()

        const allVersions = store.allVersions
        expect(allVersions.core).toBe('1.0.0')
        expect(allVersions.worker).toBe('1.1.0')
        expect(allVersions.maintenance).toBe('1.2.0')
        expect(allVersions.fe).toBeDefined()
    })

    it('should clear versions', async () => {
        const mockVersions: SystemVersions = {
            core: '1.0.0',
            worker: {
                name: 'worker',
                version: '1.0.0',
                last_seen_at: '2024-01-01T00:00:00Z',
            },
            maintenance: null,
        }

        vi.mocked(typedHttpClient.getSystemVersions).mockResolvedValue(mockVersions)

        const store = useVersionStore()
        await store.loadVersions()
        expect(store.hasVersions).toBe(true)

        store.clearVersions()

        expect(store.coreVersion).toBeNull()
        expect(store.workerVersion).toBeNull()
        expect(store.maintenanceVersion).toBeNull()
        expect(store.hasVersions).toBe(false)
        expect(store.lastFetchedAt).toBeNull()
    })

    it('should handle load error gracefully', async () => {
        vi.mocked(typedHttpClient.getSystemVersions).mockRejectedValue(new Error('Network error'))

        const store = useVersionStore()
        await store.loadVersions()

        expect(store.error).toBe('Network error')
        expect(store.isLoading).toBe(false)
        expect(store.hasVersions).toBe(false)
    })

    it('should not reload while already loading', async () => {
        const mockVersions: SystemVersions = {
            core: '1.0.0',
            worker: null,
            maintenance: null,
        }

        vi.mocked(typedHttpClient.getSystemVersions).mockResolvedValue(mockVersions)

        const store = useVersionStore()

        // Start two concurrent loads
        const load1 = store.loadVersions()
        const load2 = store.loadVersions()

        await Promise.all([load1, load2])

        // Should only call the API once due to isLoading check
        expect(typedHttpClient.getSystemVersions).toHaveBeenCalledTimes(1)
    })
})
