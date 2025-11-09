import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import EntityTree from './EntityTree.vue'

const mockBrowseByPath = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        browseByPath: (...args: any[]) => mockBrowseByPath(...args),
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
        ;(wrapper.vm as any).treeItems = treeItems
        ;(wrapper.vm as any).loadedPaths.add('/test')

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
        const reloadPath = (wrapper.vm as any).reloadPath
        await reloadPath('/test')

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
        ;(wrapper.vm as any).treeItems = treeItems
        ;(wrapper.vm as any).loadedPaths.add('/test')

        // Mock browseByPath for reload
        mockBrowseByPath.mockResolvedValueOnce({
            data: [],
        })

        // Call reloadPath
        const reloadPath = (wrapper.vm as any).reloadPath
        await reloadPath('/test')

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
        const reloadPath = (wrapper.vm as any).reloadPath
        await reloadPath('/')

        // Check that browseByPath was called with root
        expect(mockBrowseByPath).toHaveBeenCalledWith('/', 100, 0)

        // Check that expanded state was preserved
        const emitted = wrapper.emitted('update:expandedItems')
        expect(emitted).toBeDefined()
        const lastEmit = emitted?.[emitted.length - 1]?.[0] as string[]
        expect(lastEmit).toEqual(expandedItems)
    })
})
