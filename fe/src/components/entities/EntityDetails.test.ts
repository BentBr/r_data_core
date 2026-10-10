import { mount, VueWrapper } from '@vue/test-utils'
import { describe, it, expect, vi, beforeEach } from 'vitest'
import EntityDetails from './EntityDetails.vue'
import type { DynamicEntity, EntityDefinition } from '@/types/schemas'

// Mock API
vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        listEntityVersions: vi.fn(),
        getEntityVersion: vi.fn(),
    },
}))

// Mock Translations
vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (key: string) => {
            const translations: Record<string, string> = {
                'entities.details.select_entity': 'Select an entity',
                'entities.details.no_parent': 'No parent',
                'common.empty': '(empty)',
                'common.yes': 'Yes',
                'common.no': 'No',
            }
            return translations[key] ?? key
        },
    }),
}))

vi.mock('@/components/common/SmartIcon.vue', () => ({
    default: { template: '<div class="smart-icon"></div>' },
}))

const makeDefinition = (overrides: Partial<EntityDefinition> = {}): EntityDefinition =>
    ({
        uuid: 'def-uuid-1',
        entity_type: 'test_type',
        display_name: 'Test Type',
        published: true,
        allow_children: true,
        fields: [
            { name: 'first_name', display_name: 'First Name', field_type: 'String' },
            { name: 'is_active', display_name: 'Active', field_type: 'Boolean' },
            { name: 'secret', display_name: 'Secret', field_type: 'Password' },
            { name: 'born_on', display_name: 'Born On', field_type: 'Date' },
            { name: 'meta', display_name: 'Meta', field_type: 'Json' },
        ],
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        created_by: 'user-uuid',
        version: 1,
        ...overrides,
    }) as unknown as EntityDefinition

const makeEntity = (overrides: Partial<DynamicEntity> = {}): DynamicEntity =>
    ({
        entity_type: 'test_type',
        field_data: {
            uuid: 'entity-uuid-1',
            created_at: '2024-01-01T00:00:00Z',
            updated_at: '2024-01-02T00:00:00Z',
            path: '/some/path',
            parent_uuid: null,
            FirstName: 'Ada',
            isActive: true,
            secret: 'shh',
            born_on: '2024-03-04',
            meta: { nested: true },
        },
        children_count: 3,
        ...overrides,
    }) as unknown as DynamicEntity

describe('EntityDetails', () => {
    let wrapper: VueWrapper

    beforeEach(() => {
        vi.clearAllMocks()
    })

    const mountComponent = (props: {
        entity: DynamicEntity | null
        entityDefinition: EntityDefinition | null
    }) => mount(EntityDetails, { props })

    describe('empty state', () => {
        it('shows placeholder when no entity is selected', () => {
            wrapper = mountComponent({ entity: null, entityDefinition: null })

            expect(wrapper.text()).toContain('Select an entity')
            expect(wrapper.find('.entity-details').exists()).toBe(true)
        })

        it('does not call listEntityVersions when entity is null', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            mountComponent({ entity: null, entityDefinition: null })
            await Promise.resolve()

            expect(typedHttpClient.listEntityVersions).not.toHaveBeenCalled()
        })
    })

    describe('with entity selected', () => {
        it('renders basic info fields', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            wrapper = mountComponent({ entity: makeEntity(), entityDefinition: makeDefinition() })
            await wrapper.vm.$nextTick()
            await Promise.resolve()

            expect(wrapper.text()).toContain('entity-uuid-1')
            expect(wrapper.text()).toContain('test_type')
            expect(wrapper.text()).toContain('/some/path')
        })

        it('shows "no parent" message when parent_uuid is absent', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            wrapper = mountComponent({ entity: makeEntity(), entityDefinition: makeDefinition() })
            await wrapper.vm.$nextTick()

            expect(wrapper.text()).toContain('No parent')
        })

        it('shows parent uuid when present', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            wrapper = mountComponent({
                entity: makeEntity({
                    field_data: {
                        ...makeEntity().field_data,
                        parent_uuid: 'parent-uuid-9',
                    },
                }),
                entityDefinition: makeDefinition(),
            })
            await wrapper.vm.$nextTick()

            expect(wrapper.text()).toContain('parent-uuid-9')
        })

        it('shows children count', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            wrapper = mountComponent({
                entity: makeEntity({ children_count: 7 }),
                entityDefinition: makeDefinition(),
            })
            await wrapper.vm.$nextTick()

            expect(wrapper.text()).toContain('7')
        })

        it('emits edit when edit button is clicked', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            wrapper = mountComponent({ entity: makeEntity(), entityDefinition: makeDefinition() })
            await wrapper.vm.$nextTick()

            const buttons = wrapper.findAll('button')
            const editButton = buttons.find(b => b.text().includes('edit'))
            await editButton?.trigger('click')

            expect(wrapper.emitted('edit')).toBeTruthy()
        })

        it('emits delete when delete button is clicked', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            wrapper = mountComponent({ entity: makeEntity(), entityDefinition: makeDefinition() })
            await wrapper.vm.$nextTick()

            const buttons = wrapper.findAll('button')
            const deleteButton = buttons.find(b => b.text().includes('delete'))
            await deleteButton?.trigger('click')

            expect(wrapper.emitted('delete')).toBeTruthy()
        })
    })

    describe('resolveFieldValue / formatFieldValue rendering', () => {
        it('resolves case-insensitive field names', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            wrapper = mountComponent({
                entity: makeEntity(),
                entityDefinition: makeDefinition(),
            })
            await wrapper.vm.$nextTick()

            // 'first_name' resolves via token-matching to 'FirstName'
            expect(wrapper.text()).toContain('Ada')
        })

        it('masks Password field values', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            wrapper = mountComponent({
                entity: makeEntity(),
                entityDefinition: makeDefinition(),
            })
            await wrapper.vm.$nextTick()

            expect(wrapper.text()).toContain('******')
            expect(wrapper.text()).not.toContain('shh')
        })

        it('renders Boolean true as Yes', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            wrapper = mountComponent({
                entity: makeEntity(),
                entityDefinition: makeDefinition(),
            })
            await wrapper.vm.$nextTick()

            expect(wrapper.text()).toContain('Yes')
        })

        it('renders Boolean false as No', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            wrapper = mountComponent({
                entity: makeEntity({
                    field_data: { ...makeEntity().field_data, isActive: false },
                }),
                entityDefinition: makeDefinition(),
            })
            await wrapper.vm.$nextTick()

            expect(wrapper.text()).toContain('No')
        })

        it('shows empty placeholder for undefined field values', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            const definition = makeDefinition({
                fields: [
                    { name: 'unknown_field', display_name: 'Unknown', field_type: 'String' },
                ] as unknown as EntityDefinition['fields'],
            })

            wrapper = mountComponent({
                entity: makeEntity(),
                entityDefinition: definition,
            })
            await wrapper.vm.$nextTick()

            expect(wrapper.text()).toContain('(empty)')
        })

        it('stringifies Json object field values', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            wrapper = mountComponent({
                entity: makeEntity(),
                entityDefinition: makeDefinition(),
            })
            await wrapper.vm.$nextTick()

            expect(wrapper.text()).toContain('"nested":true')
        })

        it('renders raw JSON data in the raw-data panel once expanded', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            wrapper = mountComponent({
                entity: makeEntity(),
                entityDefinition: makeDefinition(),
            })
            await wrapper.vm.$nextTick()

            const titles = wrapper.findAll('.v-expansion-panel-title')
            const rawDataTitle = titles.find(t => t.text().includes('raw_data'))
            expect(rawDataTitle).toBeDefined()
            await rawDataTitle?.trigger('click')
            await wrapper.vm.$nextTick()
            await new Promise(resolve => setTimeout(resolve, 10))

            const pre = wrapper.find('pre')
            expect(pre.exists()).toBe(true)
            expect(pre.text()).toContain('entity-uuid-1')
        })
    })

    describe('version loading', () => {
        it('loads versions when entity uuid is present', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([
                { version_number: 1, created_at: '2024-01-01T00:00:00Z' },
            ])

            wrapper = mountComponent({ entity: makeEntity(), entityDefinition: makeDefinition() })
            await wrapper.vm.$nextTick()
            await Promise.resolve()

            expect(typedHttpClient.listEntityVersions).toHaveBeenCalledWith(
                'test_type',
                'entity-uuid-1'
            )
        })

        it('logs and swallows error when version loading fails', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockRejectedValue(new Error('boom'))

            wrapper = mountComponent({ entity: makeEntity(), entityDefinition: makeDefinition() })
            await wrapper.vm.$nextTick()
            await Promise.resolve()

            // Should not throw and component should still render
            expect(wrapper.find('.entity-details').exists()).toBe(true)
        })

        it('reloads versions when entity uuid changes', async () => {
            const { typedHttpClient } = await import('@/api/typed-client')
            vi.mocked(typedHttpClient.listEntityVersions).mockResolvedValue([])

            wrapper = mountComponent({ entity: makeEntity(), entityDefinition: makeDefinition() })
            await wrapper.vm.$nextTick()

            await wrapper.setProps({
                entity: makeEntity({
                    field_data: { ...makeEntity().field_data, uuid: 'entity-uuid-2' },
                }),
            })
            await wrapper.vm.$nextTick()
            await Promise.resolve()

            expect(typedHttpClient.listEntityVersions).toHaveBeenCalledWith(
                'test_type',
                'entity-uuid-2'
            )
        })
    })
})
