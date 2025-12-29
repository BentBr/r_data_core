import { describe, it, expect } from 'vitest'
import { HttpError, extractActionFromMethod, extractNamespaceFromEndpoint } from './errors'

describe('errors', () => {
    describe('HttpError', () => {
        it('should create HttpError with all properties', () => {
            const error = new HttpError(
                409,
                'user',
                'create',
                'User already exists',
                'Original message'
            )

            expect(error).toBeInstanceOf(Error)
            expect(error).toBeInstanceOf(HttpError)
            expect(error.name).toBe('HttpError')
            expect(error.statusCode).toBe(409)
            expect(error.namespace).toBe('user')
            expect(error.action).toBe('create')
            expect(error.message).toBe('User already exists')
            expect(error.originalMessage).toBe('Original message')
        })

        it('should use message as originalMessage when not provided', () => {
            const error = new HttpError(404, 'entity', 'read', 'Not found')

            expect(error.originalMessage).toBe('Not found')
        })

        it('should be throwable and catchable', () => {
            expect(() => {
                throw new HttpError(500, 'system', 'unknown', 'Server error')
            }).toThrow(HttpError)
        })
    })

    describe('extractActionFromMethod', () => {
        it('should return "create" for POST method', () => {
            expect(extractActionFromMethod('POST')).toBe('create')
            expect(extractActionFromMethod('post')).toBe('create')
        })

        it('should return "read" for GET method', () => {
            expect(extractActionFromMethod('GET')).toBe('read')
            expect(extractActionFromMethod('get')).toBe('read')
        })

        it('should return "update" for PUT method', () => {
            expect(extractActionFromMethod('PUT')).toBe('update')
            expect(extractActionFromMethod('put')).toBe('update')
        })

        it('should return "update" for PATCH method', () => {
            expect(extractActionFromMethod('PATCH')).toBe('update')
            expect(extractActionFromMethod('patch')).toBe('update')
        })

        it('should return "delete" for DELETE method', () => {
            expect(extractActionFromMethod('DELETE')).toBe('delete')
            expect(extractActionFromMethod('delete')).toBe('delete')
        })

        it('should return "read" for undefined method (default)', () => {
            expect(extractActionFromMethod(undefined)).toBe('read')
        })

        it('should return "unknown" for unrecognized methods', () => {
            expect(extractActionFromMethod('OPTIONS')).toBe('unknown')
            expect(extractActionFromMethod('HEAD')).toBe('unknown')
        })
    })

    describe('extractNamespaceFromEndpoint', () => {
        describe('admin API endpoints', () => {
            it('should extract "user" from /admin/api/v1/users', () => {
                expect(extractNamespaceFromEndpoint('/admin/api/v1/users')).toBe('user')
            })

            it('should extract "user" from /admin/api/v1/users/uuid', () => {
                expect(extractNamespaceFromEndpoint('/admin/api/v1/users/123-456')).toBe('user')
            })

            it('should extract "role" from /admin/api/v1/roles', () => {
                expect(extractNamespaceFromEndpoint('/admin/api/v1/roles')).toBe('role')
            })

            it('should extract "api_key" from /admin/api/v1/api-keys', () => {
                expect(extractNamespaceFromEndpoint('/admin/api/v1/api-keys')).toBe('api_key')
            })

            it('should extract "entity_definition" from /admin/api/v1/entity-definitions', () => {
                expect(extractNamespaceFromEndpoint('/admin/api/v1/entity-definitions')).toBe(
                    'entity_definition'
                )
            })

            it('should extract "workflow" from /admin/api/v1/workflows', () => {
                expect(extractNamespaceFromEndpoint('/admin/api/v1/workflows')).toBe('workflow')
            })

            it('should extract "workflow" from /admin/api/v1/workflows/uuid', () => {
                expect(extractNamespaceFromEndpoint('/admin/api/v1/workflows/abc-123')).toBe(
                    'workflow'
                )
            })
        })

        describe('public API endpoints', () => {
            it('should extract "entity" from /api/v1/entities', () => {
                expect(extractNamespaceFromEndpoint('/api/v1/entities')).toBe('entity')
            })

            it('should extract "entity" from /api/v1/entities/by-path', () => {
                expect(extractNamespaceFromEndpoint('/api/v1/entities/by-path?path=/root')).toBe(
                    'entity'
                )
            })

            it('should extract "entity" from /api/v1/entities/query', () => {
                expect(extractNamespaceFromEndpoint('/api/v1/entities/query')).toBe('entity')
            })
        })

        describe('dynamic entity type endpoints', () => {
            it('should extract "entity" for dynamic entity type /api/v1/Product', () => {
                expect(extractNamespaceFromEndpoint('/api/v1/Product')).toBe('entity')
            })

            it('should extract "entity" for dynamic entity type /api/v1/Puh', () => {
                expect(extractNamespaceFromEndpoint('/api/v1/Puh')).toBe('entity')
            })

            it('should extract "entity" for dynamic entity type /api/v1/MyCustomType', () => {
                expect(extractNamespaceFromEndpoint('/api/v1/MyCustomType')).toBe('entity')
            })

            it('should extract "entity" for dynamic entity type with uuid /api/v1/Product/uuid', () => {
                expect(extractNamespaceFromEndpoint('/api/v1/Product/123-456-789')).toBe('entity')
            })

            it('should extract "entity" for snake_case entity types /api/v1/my_entity', () => {
                expect(extractNamespaceFromEndpoint('/api/v1/my_entity')).toBe('entity')
            })
        })

        describe('edge cases', () => {
            it('should handle endpoints with query parameters', () => {
                expect(extractNamespaceFromEndpoint('/admin/api/v1/users?page=1&per_page=20')).toBe(
                    'user'
                )
            })

            it('should return "unknown" for empty path', () => {
                expect(extractNamespaceFromEndpoint('/')).toBe('unknown')
            })

            it('should return "unknown" for path with only prefixes', () => {
                expect(extractNamespaceFromEndpoint('/admin/api/v1')).toBe('unknown')
            })

            it('should handle DSL endpoints', () => {
                expect(extractNamespaceFromEndpoint('/admin/api/v1/dsl/validate')).toBe('dsl')
            })

            it('should handle auth endpoints', () => {
                expect(extractNamespaceFromEndpoint('/admin/api/v1/auth/login')).toBe('auth')
            })

            it('should handle system endpoints', () => {
                expect(extractNamespaceFromEndpoint('/admin/api/v1/system/settings')).toBe('system')
            })
        })
    })
})
