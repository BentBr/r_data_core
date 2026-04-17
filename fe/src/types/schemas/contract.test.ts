/**
 * Contract tests — verify that realistic API response fixtures satisfy the
 * generated TypeScript types from Rust structs.
 *
 * If a backend struct changes (field added/removed/renamed), these tests
 * catch the drift at compile time via type assignment checks.
 *
 * Each fixture mirrors what the BE actually sends for that endpoint.
 */
import { describe, it, expect } from 'vitest'
import type { AdminLoginResponse } from '../generated/AdminLoginResponse'
import type { RefreshTokenResponse } from '../generated/RefreshTokenResponse'
import type { UserResponse } from '../generated/UserResponse'
import type { CreateUserRequest } from '../generated/CreateUserRequest'
import type { RoleResponse } from '../generated/RoleResponse'
import type { PermissionResponse } from '../generated/PermissionResponse'
import type { ApiKeyResponse } from '../generated/ApiKeyResponse'
import type { ApiKeyCreatedResponse } from '../generated/ApiKeyCreatedResponse'
import type { CreateApiKeyRequest } from '../generated/CreateApiKeyRequest'
import type { WorkflowSummary } from '../generated/WorkflowSummary'
import type { WorkflowDetail } from '../generated/WorkflowDetail'
import type { WorkflowRunSummary } from '../generated/WorkflowRunSummary'
import type { WorkflowRunLogDto } from '../generated/WorkflowRunLogDto'
import type { EntityDefinitionSchema } from '../generated/EntityDefinitionSchema'
import type { FieldDefinitionSchema } from '../generated/FieldDefinitionSchema'
import type { PaginationMeta } from '../generated/PaginationMeta'
import type { ValidationViolation } from '../generated/ValidationViolation'
import type { DslValidateResponse } from '../generated/DslValidateResponse'

// Helper: assigns a value to a typed variable. If the shape doesn't match,
// TypeScript fails at compile time. The runtime check is just a sanity assertion.
function assertType<T>(value: T): T {
    return value
}

describe('Generated type contract tests', () => {
    describe('Auth types', () => {
        it('AdminLoginResponse matches expected API shape', () => {
            const fixture = assertType<AdminLoginResponse>({
                access_token: 'eyJhbGciOiJIUzI1NiJ9.test',
                refresh_token: 'eyJhbGciOiJIUzI1NiJ9.refresh',
                user_uuid: '01923e4a-5b6c-7d8e-9f01-234567890abc',
                username: 'admin',
                access_expires_at: '2024-12-31T23:59:59Z',
                refresh_expires_at: '2025-06-30T23:59:59Z',
                using_default_password: false,
            })
            expect(fixture.access_token).toBeTruthy()
            expect(fixture.user_uuid).toBeTruthy()
        })

        it('RefreshTokenResponse matches expected API shape', () => {
            const fixture = assertType<RefreshTokenResponse>({
                access_token: 'new-access-token',
                refresh_token: 'new-refresh-token',
                access_expires_at: '2024-12-31T23:59:59Z',
                refresh_expires_at: '2025-06-30T23:59:59Z',
            })
            expect(fixture.access_token).toBeTruthy()
        })
    })

    describe('User types', () => {
        it('UserResponse matches expected API shape', () => {
            const fixture = assertType<UserResponse>({
                uuid: '01923e4a-5b6c-7d8e-9f01-234567890abc',
                username: 'john.doe',
                email: 'john@example.com',
                full_name: 'John Doe',
                first_name: 'John',
                last_name: 'Doe',
                role_uuids: ['01923e4a-aaaa-7d8e-9f01-234567890abc'],
                status: 'Active',
                is_active: true,
                is_admin: false,
                super_admin: false,
                last_login: '2024-06-15T10:30:00Z',
                failed_login_attempts: 0,
                created_at: '2024-01-01T00:00:00Z',
                updated_at: '2024-06-15T10:30:00Z',
                created_by: '01923e4a-bbbb-7d8e-9f01-234567890abc',
            })
            expect(fixture.uuid).toBeTruthy()
        })

        it('UserResponse handles null optional fields', () => {
            const fixture = assertType<UserResponse>({
                uuid: '01923e4a-5b6c-7d8e-9f01-234567890abc',
                username: 'jane',
                email: 'jane@example.com',
                full_name: 'Jane',
                first_name: null,
                last_name: null,
                role_uuids: [],
                status: 'Active',
                is_active: true,
                is_admin: false,
                super_admin: false,
                last_login: null,
                failed_login_attempts: 0,
                created_at: '2024-01-01T00:00:00Z',
                updated_at: '2024-01-01T00:00:00Z',
                created_by: '01923e4a-bbbb-7d8e-9f01-234567890abc',
            })
            expect(fixture.first_name).toBeNull()
            expect(fixture.last_login).toBeNull()
        })

        it('CreateUserRequest matches expected request shape', () => {
            const fixture = assertType<CreateUserRequest>({
                username: 'newuser',
                email: 'new@example.com',
                password: 'securepassword123',
                first_name: 'New',
                last_name: 'User',
                role_uuids: null,
                is_active: null,
                super_admin: null,
            })
            expect(fixture.username).toBeTruthy()
        })
    })

    describe('Role & Permission types', () => {
        it('RoleResponse matches expected API shape', () => {
            const fixture = assertType<RoleResponse>({
                uuid: '01923e4a-5b6c-7d8e-9f01-234567890abc',
                name: 'Editor',
                description: 'Can edit content',
                is_system: false,
                super_admin: false,
                permissions: [
                    {
                        resource_type: 'Entities',
                        permission_type: 'Update',
                        access_level: 'All',
                        resource_uuids: [],
                        constraints: null,
                    },
                ],
                created_at: '2024-01-01T00:00:00Z',
                updated_at: '2024-06-01T00:00:00Z',
                created_by: '01923e4a-bbbb-7d8e-9f01-234567890abc',
                updated_by: null,
                published: true,
                version: 1,
            })
            expect(fixture.permissions).toHaveLength(1)
        })

        it('PermissionResponse matches expected shape', () => {
            const fixture = assertType<PermissionResponse>({
                resource_type: 'Workflows',
                permission_type: 'Execute',
                access_level: 'All',
                resource_uuids: ['01923e4a-cccc-7d8e-9f01-234567890abc'],
                constraints: null,
            })
            expect(fixture.resource_type).toBe('Workflows')
        })
    })

    describe('API Key types', () => {
        it('ApiKeyResponse matches expected API shape', () => {
            const fixture = assertType<ApiKeyResponse>({
                uuid: '01923e4a-5b6c-7d8e-9f01-234567890abc',
                name: 'Production Key',
                description: 'Used by CI pipeline',
                is_active: true,
                created_at: '2024-01-01T00:00:00Z',
                expires_at: '2025-01-01T00:00:00Z',
                last_used_at: '2024-06-15T10:30:00Z',
                created_by: '01923e4a-bbbb-7d8e-9f01-234567890abc',
                user_uuid: '01923e4a-cccc-7d8e-9f01-234567890abc',
                published: true,
            })
            expect(fixture.name).toBe('Production Key')
        })

        it('ApiKeyCreatedResponse includes the actual key value', () => {
            const fixture = assertType<ApiKeyCreatedResponse>({
                uuid: '01923e4a-5b6c-7d8e-9f01-234567890abc',
                name: 'New Key',
                api_key: 'rdc_live_abc123def456',
                description: null,
                is_active: true,
                created_at: '2024-01-01T00:00:00Z',
                expires_at: null,
                created_by: '01923e4a-bbbb-7d8e-9f01-234567890abc',
                user_uuid: '01923e4a-cccc-7d8e-9f01-234567890abc',
                published: true,
                last_used_at: null,
            })
            expect(fixture.api_key).toBeTruthy()
        })

        it('CreateApiKeyRequest matches expected request shape', () => {
            const fixture = assertType<CreateApiKeyRequest>({
                name: 'Test Key',
                description: null,
                expires_in_days: 365,
            })
            expect(fixture.name).toBe('Test Key')
        })
    })

    describe('Workflow types', () => {
        it('WorkflowSummary matches expected API shape', () => {
            const fixture = assertType<WorkflowSummary>({
                uuid: '01923e4a-5b6c-7d8e-9f01-234567890abc',
                name: 'Import Products',
                kind: 'Consumer',
                enabled: true,
                schedule_cron: '0 */6 * * *',
                has_api_endpoint: false,
                versioning_disabled: false,
            })
            expect(fixture.kind).toBe('Consumer')
        })

        it('WorkflowDetail matches expected API shape', () => {
            const fixture = assertType<WorkflowDetail>({
                uuid: '01923e4a-5b6c-7d8e-9f01-234567890abc',
                name: 'Import Products',
                description: 'Imports product data from CSV',
                kind: 'Consumer',
                enabled: true,
                schedule_cron: '0 */6 * * *',
                config: { steps: [] },
                versioning_disabled: false,
            })
            expect(fixture.config).toBeDefined()
        })

        it('WorkflowRunSummary matches expected API shape', () => {
            const fixture = assertType<WorkflowRunSummary>({
                uuid: '01923e4a-5b6c-7d8e-9f01-234567890abc',
                status: 'completed',
                queued_at: '2024-06-15T10:00:00Z',
                started_at: '2024-06-15T10:00:05Z',
                finished_at: '2024-06-15T10:05:00Z',
                processed_items: 150,
                failed_items: 2,
            })
            expect(fixture.processed_items).toBe(150)
        })

        it('WorkflowRunLogDto matches expected API shape', () => {
            const fixture = assertType<WorkflowRunLogDto>({
                uuid: '01923e4a-5b6c-7d8e-9f01-234567890abc',
                ts: '2024-06-15T10:00:05Z',
                level: 'info',
                message: 'Processing started',
                meta: { batch_size: 50 },
            })
            expect(fixture.level).toBe('info')
        })
    })

    describe('Entity Definition types', () => {
        it('FieldDefinitionSchema matches expected API shape', () => {
            const fixture = assertType<FieldDefinitionSchema>({
                name: 'product_name',
                display_name: 'Product Name',
                field_type: 'String',
                description: 'The product display name',
                required: true,
                indexed: true,
                filterable: true,
                unique: false,
                default_value: null,
                constraints: null,
                ui_settings: {
                    placeholder: null,
                    help_text: null,
                    hide_in_lists: null,
                    width: null,
                    order: null,
                    group: null,
                    css_class: null,
                    wysiwyg_toolbar: null,
                    input_type: null,
                },
            })
            expect(fixture.field_type).toBe('String')
        })

        it('EntityDefinitionSchema matches expected API shape', () => {
            const fixture = assertType<EntityDefinitionSchema>({
                uuid: '01923e4a-5b6c-7d8e-9f01-234567890abc',
                entity_type: 'products',
                display_name: 'Products',
                description: 'Product catalog',
                group_name: 'Commerce',
                allow_children: false,
                icon: 'mdi-package',
                fields: [],
                published: true,
                created_at: '2024-01-01T00:00:00Z',
                updated_at: '2024-06-01T00:00:00Z',
            })
            expect(fixture.entity_type).toBe('products')
        })
    })

    describe('Common types', () => {
        it('PaginationMeta matches expected API shape', () => {
            const fixture = assertType<PaginationMeta>({
                total: 100,
                page: 1,
                per_page: 20,
                total_pages: 5,
                has_previous: false,
                has_next: true,
            })
            expect(fixture.total).toBe(100)
            expect(fixture.total_pages).toBe(5)
        })

        it('ValidationViolation matches expected API shape', () => {
            const fixture = assertType<ValidationViolation>({
                field: 'email',
                message: 'Invalid email format',
                code: null,
            })
            expect(fixture.field).toBe('email')
        })

        it('DslValidateResponse matches expected API shape', () => {
            const fixture = assertType<DslValidateResponse>({
                valid: true,
            })
            expect(fixture.valid).toBe(true)
        })
    })
})
