import { mount } from '@vue/test-utils'
import { describe, it, expect } from 'vitest'
import TreeView from './TreeView.vue'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'

const vuetify = createVuetify({ components, directives })

describe('TreeView', () => {
    const baseItems = [
        {
            id: 'group-1',
            title: 'Group',
            entity_type: 'group',
            icon: 'folder',
            children: [
                { id: 'child-1', title: 'Child 1', icon: 'file', entity_type: 'entity', published: true },
                { id: 'child-2', title: 'Child 2', icon: 'file', entity_type: 'entity', published: false },
            ],
        },
    ]

    it('emits expanded items on toggle', async () => {
        const wrapper = mount(TreeView, {
            props: { items: baseItems, expandedItems: [], loading: false },
            global: { plugins: [vuetify] },
        })

        const toggle = wrapper.findAllComponents({ name: 'SmartIcon' }).find(c =>
            ['chevron-down', 'chevron-right'].includes(c.props('icon') as string)
        )
        expect(toggle).toBeTruthy()
        await toggle?.trigger('click')
        const expanded = wrapper.emitted('update:expandedItems')
        expect(expanded?.length).toBeGreaterThan(0)
    })

    it('applies disabled styles for unpublished items (text and icon)', async () => {
        const wrapper = mount(TreeView, {
            props: { items: baseItems, expandedItems: ['group-1'], loading: false },
            global: { plugins: [vuetify] },
        })

        const rows = wrapper.findAll('.tree-row')
        expect(rows.length).toBeGreaterThanOrEqual(2)

        const publishedRow = rows.find(r => r.text().includes('Child 1'))
        const unpublishedRow = rows.find(r => r.text().includes('Child 2'))

        expect(publishedRow?.classes()).not.toContain('tree-disabled')
        expect(unpublishedRow?.classes()).toContain('tree-disabled')
        expect(unpublishedRow?.classes()).toContain('tree-strikethrough')

        // Icon disabled class
        const icons = wrapper.findAll('.tree-icon-wrapper')
        const unpublishedIcon = icons.find(i => i.classes().includes('tree-icon-disabled'))
        expect(unpublishedIcon).toBeTruthy()
    })
})

