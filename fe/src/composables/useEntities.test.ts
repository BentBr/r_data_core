import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useEntities } from './useEntities'
import { ValidationError } from '@/api/typed-client'
import type {
    DynamicEntity,
    EntityDefinition,
    CreateEntityRequest,
    UpdateEntityRequest,
} from '@/types/schemas'

// Mock dependencies
const mockHandleError = vi.fn()
const mockHandleSuccess = vi.fn()
const mockT = vi.fn((key: string) => key)
const mockGetEntityDefinitions = vi.fn()
const mockGetEntity = vi.fn()
const mockCreateEntity = vi.fn()
const mockUpdateEntity = vi.fn()
const mockDeleteEntity = vi.fn()

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
        getEntity: (...args: unknown[]) => mockGetEntity(...args),
        createEntity: (...args: unknown[]) => mockCreateEntity(...args),
        updateEntity: (...args: unknown[]) => mockUpdateEntity(...args),
        deleteEntity: (...args: unknown[]) => mockDeleteEntity(...args),
    },
    ValidationError: class ValidationError extends Error {
        violations: Array<{ field: string; message: string }>

        constructor(message: string, violations: Array<{ field: string; message: string }>) {
            super(message)
            this.violations = violations
        }
    },
}))

describe('useEntities', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockGetEntityDefinitions.mockResolvedValue({ data: [] })
    })

    describe('initial state', () => {
        it('should initialize with empty state', () => {
            const {
                entities,
                entityDefinitions,
                selectedEntity,
                loading,
                creating,
                updating,
                deleting,
            } = useEntities()
            expect(entities.value).toEqual([])
            expect(entityDefinitions.value).toEqual([])
            expect(selectedEntity.value).toBeNull()
            expect(loading.value).toBe(false)
            expect(creating.value).toBe(false)
            expect(updating.value).toBe(false)
            expect(deleting.value).toBe(false)
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

            const { loadEntityDefinitions, entityDefinitions } = useEntities()
            await loadEntityDefinitions()

            expect(entityDefinitions.value).toEqual(mockDefinitions)
        })

        it('should handle loading errors', async () => {
            const error = new Error('Failed to load')
            mockGetEntityDefinitions.mockRejectedValue(error)

            const { loadEntityDefinitions, error: errorState } = useEntities()
            await loadEntityDefinitions()

            expect(errorState.value).toBe('Failed to load')
            expect(mockHandleError).toHaveBeenCalled()
        })
    })

    describe('getEntity', () => {
        it('should get entity successfully', async () => {
            const mockEntity: DynamicEntity = {
                entity_type: 'Customer',
                field_data: { uuid: 'entity-1', name: 'Test Customer' },
            }
            mockGetEntity.mockResolvedValue(mockEntity)

            const { getEntity, selectedEntity, loading } = useEntities()
            const result = await getEntity('Customer', 'entity-1')

            expect(loading.value).toBe(false)
            expect(result).toEqual(mockEntity)
            expect(selectedEntity.value).toEqual(mockEntity)
        })

        it('should handle get errors', async () => {
            const error = new Error('Not found')
            mockGetEntity.mockRejectedValue(error)

            const { getEntity, loading } = useEntities()
            const result = await getEntity('Customer', 'entity-1')

            expect(loading.value).toBe(false)
            expect(result).toBeNull()
            expect(mockHandleError).toHaveBeenCalled()
        })
    })

    describe('createEntity', () => {
        it('should create entity successfully', async () => {
            const request: CreateEntityRequest = {
                entity_type: 'Customer',
                data: { name: 'New Customer' },
            }
            mockCreateEntity.mockResolvedValue(undefined)

            const { createEntity, creating } = useEntities()
            const onSuccess = vi.fn()
            const result = await createEntity(request, onSuccess)

            expect(creating.value).toBe(false)
            expect(result).toBeNull()
            expect(onSuccess).toHaveBeenCalled()
            expect(mockHandleSuccess).toHaveBeenCalled()
        })

        it('should throw ValidationError for validation errors', async () => {
            const request: CreateEntityRequest = {
                entity_type: 'Customer',
                data: { name: '' },
            }
            const validationError = new ValidationError('Validation failed', [
                { field: 'name', message: 'Required' },
            ])
            mockCreateEntity.mockRejectedValue(validationError)

            const { createEntity, creating } = useEntities()
            await expect(createEntity(request)).rejects.toThrow(ValidationError)

            expect(creating.value).toBe(false)
        })

        it('should handle other errors', async () => {
            const request: CreateEntityRequest = {
                entity_type: 'Customer',
                data: { name: 'Test' },
            }
            const error = new Error('Creation failed')
            mockCreateEntity.mockRejectedValue(error)

            const { createEntity, creating } = useEntities()
            const result = await createEntity(request)

            expect(creating.value).toBe(false)
            expect(result).toBeNull()
            expect(mockHandleError).toHaveBeenCalled()
        })
    })

    describe('updateEntity', () => {
        it('should update entity successfully', async () => {
            const mockEntity: DynamicEntity = {
                entity_type: 'Customer',
                field_data: { uuid: 'entity-1', name: 'Old Name' },
            }
            const updateData: UpdateEntityRequest = { data: { name: 'New Name' } }
            mockUpdateEntity.mockResolvedValue(undefined)

            const { updateEntity, selectedEntity, updating } = useEntities()
            selectedEntity.value = mockEntity
            const result = await updateEntity('Customer', 'entity-1', updateData)

            expect(updating.value).toBe(false)
            expect(result).toBe(true)
            // The composable merges updateData onto selectedEntity
            expect(selectedEntity.value).toBeDefined()
            expect(mockHandleSuccess).toHaveBeenCalled()
        })

        it('should handle update errors', async () => {
            const updateData: UpdateEntityRequest = { data: { name: 'New Name' } }
            const error = new Error('Update failed')
            mockUpdateEntity.mockRejectedValue(error)

            const { updateEntity, updating } = useEntities()
            const result = await updateEntity('Customer', 'entity-1', updateData)

            expect(updating.value).toBe(false)
            expect(result).toBe(false)
            expect(mockHandleError).toHaveBeenCalled()
        })
    })

    describe('deleteEntity', () => {
        it('should delete entity successfully', async () => {
            const mockEntity: DynamicEntity = {
                entity_type: 'Customer',
                field_data: { uuid: 'entity-1', name: 'Test' },
            }
            mockDeleteEntity.mockResolvedValue(undefined)

            const { deleteEntity, entities, selectedEntity, selectedItems, deleting } =
                useEntities()
            entities.value = [mockEntity]
            selectedEntity.value = mockEntity
            selectedItems.value = ['entity-1']
            const result = await deleteEntity('Customer', 'entity-1')

            expect(deleting.value).toBe(false)
            expect(result).toBe(true)
            expect(entities.value).toEqual([])
            expect(selectedEntity.value).toBeNull()
            expect(selectedItems.value).toEqual([])
            expect(mockHandleSuccess).toHaveBeenCalled()
        })

        it('should handle delete errors', async () => {
            const error = new Error('Delete failed')
            mockDeleteEntity.mockRejectedValue(error)

            const { deleteEntity, deleting } = useEntities()
            const result = await deleteEntity('Customer', 'entity-1')

            expect(deleting.value).toBe(false)
            expect(result).toBe(false)
            expect(mockHandleError).toHaveBeenCalled()
        })
    })

    describe('getEntityDefinition', () => {
        it('should return entity definition by type', () => {
            const mockDefinition: EntityDefinition = {
                uuid: 'def-1',
                entity_type: 'Customer',
                display_name: 'Customer',
                allow_children: false,
                fields: [],
            }

            const { getEntityDefinition, entityDefinitions } = useEntities()
            entityDefinitions.value = [mockDefinition]

            const result = getEntityDefinition('Customer')
            expect(result).toEqual(mockDefinition)
        })

        it('should return null if definition not found', () => {
            const { getEntityDefinition } = useEntities()
            const result = getEntityDefinition('NonExistent')
            expect(result).toBeNull()
        })
    })

    describe('selectedEntityDefinition', () => {
        it('should return definition for selected entity', () => {
            const mockDefinition: EntityDefinition = {
                uuid: 'def-1',
                entity_type: 'Customer',
                display_name: 'Customer',
                allow_children: false,
                fields: [],
            }
            const mockEntity: DynamicEntity = {
                entity_type: 'Customer',
                field_data: { uuid: 'entity-1' },
            }

            const { selectedEntity, selectedEntityDefinition, entityDefinitions } = useEntities()
            entityDefinitions.value = [mockDefinition]
            selectedEntity.value = mockEntity

            expect(selectedEntityDefinition.value).toEqual(mockDefinition)
        })

        it('should return null if no entity selected', () => {
            const { selectedEntityDefinition } = useEntities()
            expect(selectedEntityDefinition.value).toBeNull()
        })
    })

    describe('selectedEntityUuid', () => {
        it('should return UUID of selected entity', () => {
            const mockEntity: DynamicEntity = {
                entity_type: 'Customer',
                field_data: { uuid: 'entity-1', name: 'Test' },
            }

            const { selectedEntity, selectedEntityUuid } = useEntities()
            selectedEntity.value = mockEntity

            expect(selectedEntityUuid.value).toBe('entity-1')
        })

        it('should return empty string if no entity selected', () => {
            const { selectedEntityUuid } = useEntities()
            expect(selectedEntityUuid.value).toBe('')
        })
    })
})
