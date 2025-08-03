import { mount } from '@vue/test-utils'
import { describe, it, expect, vi } from 'vitest'
import EntityDefinitionMetaInfo from './EntityDefinitionMetaInfo.vue'
import type { EntityDefinition } from '@/types/schemas'

// Mock the translations
vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (key: string) => key,
    }),
}))

describe('EntityDefinitionMetaInfo', () => {
    const mockDefinition: EntityDefinition = {
        uuid: '123e4567-e89b-12d3-a456-426614174000',
        entity_type: 'test_entity',
        display_name: 'Test Entity',
        description: 'A test entity definition',
        group_name: 'test_group',
        allow_children: true,
        icon: 'mdi-test',
        fields: [],
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        created_by: '123e4567-e89b-12d3-a456-426614174000',
        updated_by: undefined,
        published: true,
        version: 1,
    }

    it('renders entity type correctly', () => {
        const wrapper = mount(EntityDefinitionMetaInfo, {
            props: {
                definition: mockDefinition,
            },
        })

        expect(wrapper.text()).toContain('test_entity')
        expect(wrapper.text()).toContain('Test Entity')
    })

    it('renders description when provided', () => {
        const wrapper = mount(EntityDefinitionMetaInfo, {
            props: {
                definition: mockDefinition,
            },
        })

        expect(wrapper.text()).toContain('A test entity definition')
    })

    it('renders group name when provided', () => {
        const wrapper = mount(EntityDefinitionMetaInfo, {
            props: {
                definition: mockDefinition,
            },
        })

        expect(wrapper.text()).toContain('test_group')
    })

    it('does not render description when not provided', () => {
        const definitionWithoutDescription = {
            ...mockDefinition,
            description: undefined,
        }

        const wrapper = mount(EntityDefinitionMetaInfo, {
            props: {
                definition: definitionWithoutDescription,
            },
        })

        expect(wrapper.text()).not.toContain('A test entity definition')
    })

    it('does not render group name when not provided', () => {
        const definitionWithoutGroup = {
            ...mockDefinition,
            group_name: undefined,
        }

        const wrapper = mount(EntityDefinitionMetaInfo, {
            props: {
                definition: definitionWithoutGroup,
            },
        })

        expect(wrapper.text()).not.toContain('test_group')
    })

    it('formats dates correctly', () => {
        const wrapper = mount(EntityDefinitionMetaInfo, {
            props: {
                definition: mockDefinition,
            },
        })

        // Check that dates are formatted (should show as local date)
        expect(wrapper.text()).toContain('1/1/2024') // Local date format
    })

    it('shows published status correctly', () => {
        const wrapper = mount(EntityDefinitionMetaInfo, {
            props: {
                definition: mockDefinition,
            },
        })

        expect(wrapper.text()).toContain('entity_definitions.meta_info.published')
    })

    it('shows draft status when not published', () => {
        const draftDefinition = {
            ...mockDefinition,
            published: false,
        }

        const wrapper = mount(EntityDefinitionMetaInfo, {
            props: {
                definition: draftDefinition,
            },
        })

        expect(wrapper.text()).toContain('entity_definitions.meta_info.draft')
    })

    it('displays version number', () => {
        const wrapper = mount(EntityDefinitionMetaInfo, {
            props: {
                definition: mockDefinition,
            },
        })

        expect(wrapper.text()).toContain('1')
    })
})
