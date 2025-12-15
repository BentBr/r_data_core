import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useEntityDefinitions } from './useEntityDefinitions'
import type {
    EntityDefinition,
    CreateEntityDefinitionRequest,
    UpdateEntityDefinitionRequest,
    FieldDefinition,
} from '@/types/schemas'

// Mock dependencies
const mockHandleError = vi.fn()
const mockHandleSuccess = vi.fn()
const mockT = vi.fn((key: string) => key)
const mockGetEntityDefinitions = vi.fn()
const mockCreateEntityDefinition = vi.fn()
const mockUpdateEntityDefinition = vi.fn()
const mockDeleteEntityDefinition = vi.fn()

vi.mock('./useErrorHandler', () => ({
    useErrorHandler: () => ({
        handleError: mockHandleError,
        handleSuccess: mockHandleSuccess,
    }),
}))

vi.mock('./useTranslations', () => ({
    useTranslations: () => ({
        t: mockT,
    }),
}))

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getEntityDefinitions: (...args: unknown[]) => mockGetEntityDefinitions(...args),
        createEntityDefinition: (...args: unknown[]) => mockCreateEntityDefinition(...args),
        updateEntityDefinition: (...args: unknown[]) => mockUpdateEntityDefinition(...args),
        deleteEntityDefinition: (...args: unknown[]) => mockDeleteEntityDefinition(...args),
    },
}))

describe('useEntityDefinitions', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockGetEntityDefinitions.mockResolvedValue({ data: [] })
    })

    describe('initial state', () => {
        it('should initialize with empty state', () => {
            const {
                entityDefinitions,
                selectedDefinition,
                loading,
                creating,
                updating,
                deleting,
                savingChanges,
            } = useEntityDefinitions()
            expect(entityDefinitions.value).toEqual([])
            expect(selectedDefinition.value).toBeNull()
            expect(loading.value).toBe(false)
            expect(creating.value).toBe(false)
            expect(updating.value).toBe(false)
            expect(deleting.value).toBe(false)
            expect(savingChanges.value).toBe(false)
        })
    })

    describe('sanitizeFields', () => {
        it('should ensure constraints and ui_settings are objects', () => {
            const { sanitizeFields } = useEntityDefinitions()
            const fields: FieldDefinition[] = [
                {
                    name: 'test',
                    display_name: 'Test',
                    field_type: 'String',
                    required: false,
                    indexed: false,
                    filterable: false,
                    // Missing constraints and ui_settings
                },
            ]

            const sanitized = sanitizeFields(fields)
            expect(sanitized[0].constraints).toEqual({})
            expect(sanitized[0].ui_settings).toEqual({})
        })
    })

    describe('loadEntityDefinitions', () => {
        it('should load entity definitions successfully', async () => {
            const mockDefinitions: EntityDefinition[] = [
                {
                    uuid: 'def-1',
                    entity_type: 'Customer',
                    display_name: 'Customer',
                    allow_children: false,
                    fields: [],
                },
            ]
            mockGetEntityDefinitions.mockResolvedValue({ data: mockDefinitions })

            const { loadEntityDefinitions, entityDefinitions, loading } = useEntityDefinitions()
            await loadEntityDefinitions()

            expect(loading.value).toBe(false)
            expect(entityDefinitions.value.length).toBe(1)
            expect(entityDefinitions.value[0].entity_type).toBe('Customer')
        })

        it('should sanitize fields when loading', async () => {
            const mockDefinitions: EntityDefinition[] = [
                {
                    uuid: 'def-1',
                    entity_type: 'Customer',
                    display_name: 'Customer',
                    allow_children: false,
                    fields: [
                        {
                            name: 'test',
                            display_name: 'Test',
                            field_type: 'String',
                            required: false,
                            indexed: false,
                            filterable: false,
                        },
                    ],
                },
            ]
            mockGetEntityDefinitions.mockResolvedValue({ data: mockDefinitions })

            const { loadEntityDefinitions, entityDefinitions } = useEntityDefinitions()
            await loadEntityDefinitions()

            expect(entityDefinitions.value[0].fields[0].constraints).toEqual({})
            expect(entityDefinitions.value[0].fields[0].ui_settings).toEqual({})
        })

        it('should handle loading errors', async () => {
            const error = new Error('Failed to load')
            mockGetEntityDefinitions.mockRejectedValue(error)

            const { loadEntityDefinitions, error: errorState, loading } = useEntityDefinitions()
            await loadEntityDefinitions()

            expect(loading.value).toBe(false)
            expect(errorState.value).toBe('Failed to load')
            expect(mockHandleError).toHaveBeenCalled()
        })
    })

    describe('createEntityDefinition', () => {
        it('should create entity definition successfully', async () => {
            const request: CreateEntityDefinitionRequest = {
                entity_type: 'Customer',
                display_name: 'Customer',
                allow_children: false,
                fields: [],
            }
            mockCreateEntityDefinition.mockResolvedValue(undefined)
            mockGetEntityDefinitions.mockResolvedValue({ data: [] })

            const { createEntityDefinition, creating } = useEntityDefinitions()
            const result = await createEntityDefinition(request)

            expect(creating.value).toBe(false)
            expect(result).toBe(true)
            expect(mockHandleSuccess).toHaveBeenCalled()
        })

        it('should handle creation errors', async () => {
            const request: CreateEntityDefinitionRequest = {
                entity_type: 'Customer',
                display_name: 'Customer',
                allow_children: false,
                fields: [],
            }
            const error = new Error('Creation failed')
            mockCreateEntityDefinition.mockRejectedValue(error)

            const { createEntityDefinition, creating } = useEntityDefinitions()
            const result = await createEntityDefinition(request)

            expect(creating.value).toBe(false)
            expect(result).toBe(false)
            expect(mockHandleError).toHaveBeenCalled()
        })
    })

    describe('updateEntityDefinition', () => {
        it('should update entity definition successfully', async () => {
            const mockDefinition: EntityDefinition = {
                uuid: 'def-1',
                entity_type: 'Customer',
                display_name: 'Customer',
                allow_children: false,
                fields: [],
            }
            const updateData: UpdateEntityDefinitionRequest = {
                entity_type: 'Customer',
                display_name: 'Updated Customer',
                allow_children: false,
                fields: [],
            }
            mockUpdateEntityDefinition.mockResolvedValue(undefined)

            const { updateEntityDefinition, selectedDefinition, updating } = useEntityDefinitions()
            selectedDefinition.value = mockDefinition
            const result = await updateEntityDefinition(updateData)

            expect(updating.value).toBe(false)
            expect(result).toBe(true)
            expect(selectedDefinition.value?.display_name).toBe('Updated Customer')
            expect(mockHandleSuccess).toHaveBeenCalled()
        })

        it('should return false if no definition selected', async () => {
            const updateData: UpdateEntityDefinitionRequest = {
                entity_type: 'Customer',
                display_name: 'Updated',
                allow_children: false,
                fields: [],
            }

            const { updateEntityDefinition, updating } = useEntityDefinitions()
            const result = await updateEntityDefinition(updateData)

            expect(updating.value).toBe(false)
            expect(result).toBe(false)
            expect(mockUpdateEntityDefinition).not.toHaveBeenCalled()
        })

        it('should handle update errors', async () => {
            const mockDefinition: EntityDefinition = {
                uuid: 'def-1',
                entity_type: 'Customer',
                display_name: 'Customer',
                allow_children: false,
                fields: [],
            }
            const updateData: UpdateEntityDefinitionRequest = {
                entity_type: 'Customer',
                display_name: 'Updated',
                allow_children: false,
                fields: [],
            }
            const error = new Error('Update failed')
            mockUpdateEntityDefinition.mockRejectedValue(error)

            const { updateEntityDefinition, selectedDefinition, updating } = useEntityDefinitions()
            selectedDefinition.value = mockDefinition
            const result = await updateEntityDefinition(updateData)

            expect(updating.value).toBe(false)
            expect(result).toBe(false)
            expect(mockHandleError).toHaveBeenCalled()
        })
    })

    describe('deleteEntityDefinition', () => {
        it('should delete entity definition successfully', async () => {
            const mockDefinition: EntityDefinition = {
                uuid: 'def-1',
                entity_type: 'Customer',
                display_name: 'Customer',
                allow_children: false,
                fields: [],
            }
            mockDeleteEntityDefinition.mockResolvedValue(undefined)
            mockGetEntityDefinitions.mockResolvedValue({ data: [] })

            const { deleteEntityDefinition, selectedDefinition, deleting } = useEntityDefinitions()
            selectedDefinition.value = mockDefinition
            const result = await deleteEntityDefinition()

            expect(deleting.value).toBe(false)
            expect(result).toBe(true)
            expect(selectedDefinition.value).toBeNull()
            expect(mockHandleSuccess).toHaveBeenCalled()
        })

        it('should return false if no definition selected', async () => {
            const { deleteEntityDefinition, deleting } = useEntityDefinitions()
            const result = await deleteEntityDefinition()

            expect(deleting.value).toBe(false)
            expect(result).toBe(false)
            expect(mockDeleteEntityDefinition).not.toHaveBeenCalled()
        })

        it('should handle delete errors', async () => {
            const mockDefinition: EntityDefinition = {
                uuid: 'def-1',
                entity_type: 'Customer',
                display_name: 'Customer',
                allow_children: false,
                fields: [],
            }
            const error = new Error('Delete failed')
            mockDeleteEntityDefinition.mockRejectedValue(error)

            const { deleteEntityDefinition, selectedDefinition, deleting } = useEntityDefinitions()
            selectedDefinition.value = mockDefinition
            const result = await deleteEntityDefinition()

            expect(deleting.value).toBe(false)
            expect(result).toBe(false)
            expect(mockHandleError).toHaveBeenCalled()
        })
    })
})
