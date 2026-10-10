import { mount, VueWrapper } from '@vue/test-utils'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import EntityTree from './EntityTree.vue'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import { typedHttpClient } from '@/api/typed-client'
import type { EntityDefinition, TreeNode } from '@/types/schemas'

const vuetify = createVuetify({ components, directives })

// Mock API
vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        browseByPath: vi.fn(),
    },
}))

// Mock Translations
vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (key: string) => key }),
}))

// Mock SmartIcon to avoid rendering issues
vi.mock('@/components/common/SmartIcon.vue', () => ({
    default: { template: '<div class="smart-icon"></div>' },
}))

// Mock TreeView to inspect props and simulate emits
vi.mock('@/components/common/TreeView.vue', () => ({
    default: {
        name: 'TreeView',
        props: ['items', 'loading', 'expandedItems'],
        emits: ['update:expandedItems', 'item-click', 'selection-change'],
        template: '<div class="tree-view-mock"></div>',
    },
}))

type EntityTreeInternal = {
    reloadPath: (path: string) => Promise<void>
    expandAll: () => void
    collapseAll: () => void
}

const browseResponse = (
    nodes: Array<{
        kind: 'folder' | 'file'
        name: string
        path: string
        entity_uuid?: string | null
        entity_type?: string | null
        has_children?: boolean | null
        published?: boolean
    }>
) =>
    ({
        data: nodes,
        status: 'Success',
        message: '',
        meta: { total: nodes.length, pages: 1, limit: 100 },
    }) as any

describe('EntityTree', () => {
    let wrapper: VueWrapper

    beforeEach(() => {
        vi.clearAllMocks()
    })

    const mountComponent = (props: Record<string, unknown> = {}) =>
        mount(EntityTree, {
            global: { plugins: [vuetify] },
            props: {
                entityDefinitions: [
                    {
                        entity_type: 'test_type',
                        icon: 'file',
                        published: true,
                    } as unknown as EntityDefinition,
                ],
                ...props,
            },
        })

    it('renders published status correctly from API response', async () => {
        vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(
            browseResponse([
                {
                    kind: 'file',
                    name: 'pub-entity',
                    path: '/pub-entity',
                    entity_uuid: 'uuid-1',
                    entity_type: 'test_type',
                    has_children: false,
                    published: true,
                },
                {
                    kind: 'file',
                    name: 'unpub-entity',
                    path: '/unpub-entity',
                    entity_uuid: 'uuid-2',
                    entity_type: 'test_type',
                    has_children: false,
                    published: false,
                },
            ])
        )

        wrapper = mountComponent()
        await wrapper.setProps({ refreshKey: 1 })
        await new Promise(resolve => setTimeout(resolve, 10))

        const treeView = wrapper.findComponent({ name: 'TreeView' })
        expect(treeView.exists()).toBe(true)

        const items = treeView.props('items') as Array<{ title: string; published: boolean }>
        expect(items).toHaveLength(2)

        const pubItem = items.find(i => i.title === 'pub-entity')
        const unpubItem = items.find(i => i.title === 'unpub-entity')

        expect(pubItem?.published).toBe(true)
        expect(unpubItem?.published).toBe(false)
    })

    it('shows a loading indicator while loading and no items are present', () => {
        wrapper = mountComponent({ loading: true })

        expect(wrapper.text()).toContain('entities.tree.loading')
        expect(wrapper.findComponent({ name: 'TreeView' }).exists()).toBe(false)
    })

    it('shows an empty state when not loading and there are no entities', async () => {
        vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(browseResponse([]))

        wrapper = mountComponent()
        await wrapper.setProps({ refreshKey: 1 })
        await new Promise(resolve => setTimeout(resolve, 10))

        expect(wrapper.text()).toContain('entities.tree.no_entities')
    })

    it('emits item-click when TreeView emits item-click', async () => {
        vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(
            browseResponse([
                {
                    kind: 'file',
                    name: 'entity1',
                    path: '/entity1',
                    entity_uuid: 'uuid-1',
                    entity_type: 'test_type',
                    has_children: false,
                    published: true,
                },
            ])
        )

        wrapper = mountComponent()
        await wrapper.setProps({ refreshKey: 1 })
        await new Promise(resolve => setTimeout(resolve, 10))

        const treeView = wrapper.findComponent({ name: 'TreeView' })
        const clickedItem: TreeNode = { id: 'uuid-1', title: 'entity1' } as TreeNode
        treeView.vm.$emit('item-click', clickedItem)
        await wrapper.vm.$nextTick()

        expect(wrapper.emitted('item-click')).toBeTruthy()
        expect(wrapper.emitted('item-click')?.[0]).toEqual([clickedItem])
    })

    describe('expandAll / collapseAll', () => {
        it('expandAll emits all node ids including nested children', async () => {
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(
                browseResponse([
                    {
                        kind: 'folder',
                        name: 'folder1',
                        path: '/folder1',
                        has_children: true,
                    },
                    {
                        kind: 'file',
                        name: 'entity1',
                        path: '/entity1',
                        entity_uuid: 'uuid-1',
                        entity_type: 'test_type',
                        has_children: false,
                        published: true,
                    },
                ])
            )

            wrapper = mountComponent()
            await wrapper.setProps({ refreshKey: 1 })
            await new Promise(resolve => setTimeout(resolve, 10))

            const vm = wrapper.vm as unknown as EntityTreeInternal
            vm.expandAll()
            await wrapper.vm.$nextTick()

            const emitted = wrapper.emitted('update:expandedItems')
            expect(emitted).toBeTruthy()
            const ids = emitted?.[emitted.length - 1]?.[0] as string[]
            expect(ids).toContain('folder:/folder1')
            expect(ids).toContain('uuid-1')
        })

        it('collapseAll emits an empty array', () => {
            wrapper = mountComponent()

            const vm = wrapper.vm as unknown as EntityTreeInternal
            vm.collapseAll()

            const emitted = wrapper.emitted('update:expandedItems')
            expect(emitted?.[emitted.length - 1]).toEqual([[]])
        })
    })

    describe('reloadPath', () => {
        it('reloads the root and preserves expanded items', async () => {
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(browseResponse([]))

            wrapper = mountComponent({ expandedItems: ['a', 'b'] })
            await wrapper.setProps({ refreshKey: 1 })
            await new Promise(resolve => setTimeout(resolve, 10))

            vi.mocked(typedHttpClient.browseByPath).mockClear()
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(
                browseResponse([
                    {
                        kind: 'file',
                        name: 'new-entity',
                        path: '/new-entity',
                        entity_uuid: 'uuid-new',
                        entity_type: 'test_type',
                        has_children: false,
                        published: true,
                    },
                ])
            )

            const vm = wrapper.vm as unknown as EntityTreeInternal
            await vm.reloadPath('/')

            expect(typedHttpClient.browseByPath).toHaveBeenCalledWith('/', 100, 0)
            const emitted = wrapper.emitted('update:expandedItems')
            expect(emitted?.[emitted.length - 1]).toEqual([['a', 'b']])
        })

        it('reloads a known folder node and restores its expanded state', async () => {
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(
                browseResponse([
                    {
                        kind: 'folder',
                        name: 'folder1',
                        path: '/folder1',
                        has_children: true,
                    },
                ])
            )

            wrapper = mountComponent({ expandedItems: ['folder:/folder1'] })
            await wrapper.setProps({ refreshKey: 1 })
            await new Promise(resolve => setTimeout(resolve, 10))

            vi.mocked(typedHttpClient.browseByPath).mockClear()
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(
                browseResponse([
                    {
                        kind: 'file',
                        name: 'child1',
                        path: '/folder1/child1',
                        entity_uuid: 'uuid-child',
                        entity_type: 'test_type',
                        has_children: false,
                        published: true,
                    },
                ])
            )

            const vm = wrapper.vm as unknown as EntityTreeInternal
            await vm.reloadPath('/folder1')

            expect(typedHttpClient.browseByPath).toHaveBeenCalledWith('/folder1', 100, 0)
            const emitted = wrapper.emitted('update:expandedItems')
            expect(emitted?.[emitted.length - 1]).toEqual([['folder:/folder1']])
        })

        it('recurses into the parent path when the node is not found', async () => {
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(browseResponse([]))

            wrapper = mountComponent()
            await wrapper.setProps({ refreshKey: 1 })
            await new Promise(resolve => setTimeout(resolve, 10))

            vi.mocked(typedHttpClient.browseByPath).mockClear()
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(browseResponse([]))

            const vm = wrapper.vm as unknown as EntityTreeInternal
            // '/missing/nested' isn't in the (empty) tree; its parent is
            // '/missing', which is also not root, so it recurses once more
            // and eventually falls back to reloading '/'.
            await vm.reloadPath('/missing/nested')

            expect(typedHttpClient.browseByPath).toHaveBeenCalledWith('/', 100, 0)
        })

        it('falls back to reloading root when the parent path resolves to root', async () => {
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(browseResponse([]))

            wrapper = mountComponent()
            await wrapper.setProps({ refreshKey: 1 })
            await new Promise(resolve => setTimeout(resolve, 10))

            vi.mocked(typedHttpClient.browseByPath).mockClear()
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(browseResponse([]))

            const vm = wrapper.vm as unknown as EntityTreeInternal
            await vm.reloadPath('/top-level')

            expect(typedHttpClient.browseByPath).toHaveBeenCalledWith('/', 100, 0)
        })
    })

    describe('handleExpandedItemsChange', () => {
        it('loads children for a newly expanded folder with no loaded children yet', async () => {
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(
                browseResponse([
                    {
                        kind: 'folder',
                        name: 'folder1',
                        path: '/folder1',
                        has_children: true,
                    },
                ])
            )

            wrapper = mountComponent()
            await wrapper.setProps({ refreshKey: 1 })
            await new Promise(resolve => setTimeout(resolve, 10))

            vi.mocked(typedHttpClient.browseByPath).mockClear()
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(
                browseResponse([
                    {
                        kind: 'file',
                        name: 'child1',
                        path: '/folder1/child1',
                        entity_uuid: 'uuid-child',
                        entity_type: 'test_type',
                        has_children: false,
                        published: true,
                    },
                ])
            )

            const treeView = wrapper.findComponent({ name: 'TreeView' })
            treeView.vm.$emit('update:expandedItems', ['folder:/folder1'])
            await wrapper.vm.$nextTick()
            await new Promise(resolve => setTimeout(resolve, 10))

            expect(typedHttpClient.browseByPath).toHaveBeenCalledWith('/folder1', 100, 0)
            const emitted = wrapper.emitted('update:expandedItems')
            expect(emitted?.[emitted.length - 1]).toEqual([['folder:/folder1']])
        })

        it('logs and swallows errors when loading children for a node fails', async () => {
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(
                browseResponse([
                    {
                        kind: 'folder',
                        name: 'folder1',
                        path: '/folder1',
                        has_children: true,
                    },
                ])
            )

            wrapper = mountComponent()
            await wrapper.setProps({ refreshKey: 1 })
            await new Promise(resolve => setTimeout(resolve, 10))

            vi.mocked(typedHttpClient.browseByPath).mockClear()
            vi.mocked(typedHttpClient.browseByPath).mockRejectedValue(new Error('network error'))

            const treeView = wrapper.findComponent({ name: 'TreeView' })
            treeView.vm.$emit('update:expandedItems', ['folder:/folder1'])
            await wrapper.vm.$nextTick()
            await new Promise(resolve => setTimeout(resolve, 10))

            // Should not throw; the tree remains usable.
            expect(wrapper.findComponent({ name: 'TreeView' }).exists()).toBe(true)
        })
    })

    describe('rootPath watcher', () => {
        it('reloads the tree when rootPath changes', async () => {
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(browseResponse([]))

            wrapper = mountComponent({ rootPath: '/' })
            await wrapper.setProps({ refreshKey: 1 })
            await new Promise(resolve => setTimeout(resolve, 10))

            vi.mocked(typedHttpClient.browseByPath).mockClear()
            vi.mocked(typedHttpClient.browseByPath).mockResolvedValue(
                browseResponse([
                    {
                        kind: 'file',
                        name: 'other-entity',
                        path: '/other/other-entity',
                        entity_uuid: 'uuid-other',
                        entity_type: 'test_type',
                        has_children: false,
                        published: true,
                    },
                ])
            )

            await wrapper.setProps({ rootPath: '/other' })
            await new Promise(resolve => setTimeout(resolve, 10))

            expect(typedHttpClient.browseByPath).toHaveBeenCalledWith('/other', 100, 0)
        })
    })
})
