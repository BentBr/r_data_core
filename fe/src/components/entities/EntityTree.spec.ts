import { mount } from '@vue/test-utils'
import { describe, it, expect, vi } from 'vitest'
import EntityTree from './EntityTree.vue'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import { typedHttpClient } from '@/api/typed-client'

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

// Mock TreeView to inspect props
vi.mock('@/components/common/TreeView.vue', () => ({
    default: {
        name: 'TreeView',
        props: ['items', 'loading', 'expandedItems'],
        template: '<div class="tree-view-mock"></div>',
    },
}))

describe('EntityTree', () => {
    it('renders published status correctly from API response', async () => {
        const mockNodes = [
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
        ]

        vi.mocked(typedHttpClient.browseByPath).mockResolvedValue({
            data: mockNodes,
            status: 'Success',
            message: '',
            meta: { total: 2, pages: 1, limit: 100 },
        } as any)

        const wrapper = mount(EntityTree, {
            global: { plugins: [vuetify] },
            props: {
                entityDefinitions: [
                    { entity_type: 'test_type', icon: 'file', published: true } as any,
                ],
            },
        })

        // Trigger load via refreshKey change
        await wrapper.setProps({ refreshKey: 1 })

        // Wait for promises to resolve
        await new Promise(resolve => setTimeout(resolve, 10))

        // Find TreeView component
        const treeView = wrapper.findComponent({ name: 'TreeView' })
        expect(treeView.exists()).toBe(true)

        // Check items prop passed to TreeView
        const items = treeView.props('items')
        expect(items).toHaveLength(2)

        // Sort items to ensure order (backend usually sorts, but mock might not)
        // The component implementation pushes folders then files.
        // Both are files here.

        const pubItem = items.find((i: any) => i.title === 'pub-entity')
        const unpubItem = items.find((i: any) => i.title === 'unpub-entity')

        expect(pubItem).toBeDefined()
        expect(unpubItem).toBeDefined()

        expect(pubItem.published).toBe(true)
        expect(unpubItem.published).toBe(false)
    })

})
