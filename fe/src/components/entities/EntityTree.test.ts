import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import EntityTree from './EntityTree.vue'
import type { TreeNode } from '@/types/schemas'

const mockBrowseByPath = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        browseByPath: (path: string, limit: number, offset: number) =>
            mockBrowseByPath(path, limit, offset),
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k.split('.').pop() }),
}))

describe('EntityTree', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockBrowseByPath.mockResolvedValue({
            data: [
                {
                    kind: 'folder',
                    name: 'test',
                    path: '/test',
                    has_children: true,
                },
            ],
        })
    })

    it('reloadPath preserves expanded state when reloading a path', async () => {
        const expandedItems = ['folder:/test']
        const wrapper = mount(EntityTree, {
            props: {
                rootPath: '/',
                expandedItems,
                refreshKey: 1,
            },
        })

        // Wait for initial load
        await vi.waitUntil(() => mockBrowseByPath.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Set up tree with test folder expanded
        const treeItems = [
            {
                id: 'folder:/test',
                title: 'test',
                icon: 'mdi-folder',
                path: '/test',
                children: [
                    {
                        id: 'entity-1',
                        title: 'entity1',
                        path: '/test/entity1',
                    },
                ],
            },
        ]
        // Access component internals for testing
        const vm1 = wrapper.vm as {
            treeItems: TreeNode[]
            loadedPaths: Set<string>
            reloadPath: (path: string) => Promise<void>
        }
        vm1.treeItems = treeItems
        vm1.loadedPaths.add('/test')

        // Mock browseByPath for reload
        mockBrowseByPath.mockResolvedValueOnce({
            data: [
                {
                    kind: 'file',
                    name: 'entity2',
                    path: '/test/entity2',
                    entity_uuid: 'entity-2',
                    entity_type: 'Customer',
                },
            ],
        })

        // Call reloadPath
        await vm1.reloadPath('/test')

        // Check that expanded state was preserved
        const emitted = wrapper.emitted('update:expandedItems')
        expect(emitted).toBeDefined()
        const lastEmit = emitted?.[emitted.length - 1]?.[0] as string[]
        expect(lastEmit).toContain('folder:/test')
    })

    it('reloadPath calls browseByPath with correct path', async () => {
        const wrapper = mount(EntityTree, {
            props: {
                rootPath: '/',
                refreshKey: 1,
            },
        })

        // Wait for initial load
        await vi.waitUntil(() => mockBrowseByPath.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Set up tree with test folder
        const treeItems = [
            {
                id: 'folder:/test',
                title: 'test',
                icon: 'mdi-folder',
                path: '/test',
                children: [],
            },
        ]
        // Access component internals for testing
        const vm2 = wrapper.vm as {
            treeItems: TreeNode[]
            loadedPaths: Set<string>
            reloadPath: (path: string) => Promise<void>
        }
        vm2.treeItems = treeItems
        vm2.loadedPaths.add('/test')

        // Mock browseByPath for reload
        mockBrowseByPath.mockResolvedValueOnce({
            data: [],
        })

        // Call reloadPath
        await vm2.reloadPath('/test')

        // Check that browseByPath was called with /test
        expect(mockBrowseByPath).toHaveBeenCalledWith('/test', 100, 0)
    })

    it('reloadPath handles root path correctly', async () => {
        const expandedItems = ['folder:/test']
        const wrapper = mount(EntityTree, {
            props: {
                rootPath: '/',
                expandedItems,
                refreshKey: 1,
            },
        })

        // Wait for initial load
        await vi.waitUntil(() => mockBrowseByPath.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Mock browseByPath for root reload
        mockBrowseByPath.mockResolvedValueOnce({
            data: [],
        })

        // Call reloadPath with root
        const vm = wrapper.vm as unknown as {
            reloadPath: (path: string) => Promise<void>
        }
        await vm.reloadPath('/')

        // Check that browseByPath was called with root
        expect(mockBrowseByPath).toHaveBeenCalledWith('/', 100, 0)

        // Check that expanded state was preserved
        const emitted = wrapper.emitted('update:expandedItems')
        expect(emitted).toBeDefined()
        const lastEmit = emitted?.[emitted.length - 1]?.[0] as string[]
        expect(lastEmit).toEqual(expandedItems)
    })
})
