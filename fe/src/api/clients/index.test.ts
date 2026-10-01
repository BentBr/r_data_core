import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { TypedHttpClient } from './index'
import { EntityDefinitionsClient } from './entity-definitions'
import { ApiKeysClient } from './api-keys'
import { WorkflowsClient } from './workflows'
import { AuthClient } from './auth'
import { UsersClient } from './users'
import { EntitiesClient } from './entities'
import { SystemClient } from './system'
import { RolesClient } from './roles'
import { MetaClient } from './meta'
import { CapabilitiesClient } from './capabilities'
import { EmailTemplateClient } from './email-templates'
import { SystemLogClient } from './system-logs'

vi.mock('@/env-check', () => ({
    env: {
        apiBaseUrl: 'http://localhost:3000',
        enableApiLogging: false,
        devMode: false,
        defaultPageSize: 10,
    },
    buildApiUrl: (endpoint: string) => `http://localhost:3000${endpoint}`,
}))

// Breaks a real circular import (clients/index -> clients/base -> stores/auth ->
// api/typed-client -> clients/index) that otherwise throws "TypedHttpClient is
// not a constructor" when this module graph is loaded without the store mocked.
vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        token: null,
        refreshTokens: vi.fn(),
        logout: vi.fn(),
    }),
}))
vi.mock('@/utils/cookies', () => ({
    getRefreshToken: () => null,
}))

/**
 * `TypedHttpClient` is a pure delegation facade: every method forwards its
 * arguments verbatim to one private domain-client instance and returns
 * whatever that call resolves to. These tests prove the wiring — the right
 * domain client receives the right arguments, and the facade's own method
 * does nothing except hand back that result — by spying directly on the
 * domain client's prototype method so the real HTTP/auth plumbing never runs.
 */

/** An object standing in for data the mocked method doesn't need to inspect. */
function arg<T>(): T {
    return { __placeholder: true } as unknown as T
}

/** A unique return value, used to prove the facade returns exactly what the domain client resolved. */
function sentinel<T>(): T {
    return { __sentinel: Symbol('sentinel') } as unknown as T
}

describe('TypedHttpClient (delegation facade)', () => {
    let client: TypedHttpClient

    beforeEach(() => {
        client = new TypedHttpClient()
    })

    afterEach(() => {
        vi.restoreAllMocks()
    })

    describe('Entity Definitions', () => {
        it('delegates getEntityDefinitions', async () => {
            const expected =
                sentinel<Awaited<ReturnType<EntityDefinitionsClient['getEntityDefinitions']>>>()
            const spy = vi
                .spyOn(EntityDefinitionsClient.prototype, 'getEntityDefinitions')
                .mockResolvedValue(expected)

            const result = await client.getEntityDefinitions(10, 5)

            expect(spy).toHaveBeenCalledWith(10, 5)
            expect(result).toBe(expected)
        })

        it('delegates getEntityDefinition', async () => {
            const expected =
                sentinel<Awaited<ReturnType<EntityDefinitionsClient['getEntityDefinition']>>>()
            const spy = vi
                .spyOn(EntityDefinitionsClient.prototype, 'getEntityDefinition')
                .mockResolvedValue(expected)

            const result = await client.getEntityDefinition('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates createEntityDefinition', async () => {
            const data = arg<Parameters<EntityDefinitionsClient['createEntityDefinition']>[0]>()
            const expected =
                sentinel<Awaited<ReturnType<EntityDefinitionsClient['createEntityDefinition']>>>()
            const spy = vi
                .spyOn(EntityDefinitionsClient.prototype, 'createEntityDefinition')
                .mockResolvedValue(expected)

            const result = await client.createEntityDefinition(data)

            expect(spy).toHaveBeenCalledWith(data)
            expect(result).toBe(expected)
        })

        it('delegates updateEntityDefinition', async () => {
            const data = arg<Parameters<EntityDefinitionsClient['updateEntityDefinition']>[1]>()
            const expected =
                sentinel<Awaited<ReturnType<EntityDefinitionsClient['updateEntityDefinition']>>>()
            const spy = vi
                .spyOn(EntityDefinitionsClient.prototype, 'updateEntityDefinition')
                .mockResolvedValue(expected)

            const result = await client.updateEntityDefinition('uuid-1', data)

            expect(spy).toHaveBeenCalledWith('uuid-1', data)
            expect(result).toBe(expected)
        })

        it('delegates deleteEntityDefinition', async () => {
            const expected =
                sentinel<Awaited<ReturnType<EntityDefinitionsClient['deleteEntityDefinition']>>>()
            const spy = vi
                .spyOn(EntityDefinitionsClient.prototype, 'deleteEntityDefinition')
                .mockResolvedValue(expected)

            const result = await client.deleteEntityDefinition('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates applyEntityDefinitionSchema', async () => {
            const expected =
                sentinel<
                    Awaited<ReturnType<EntityDefinitionsClient['applyEntityDefinitionSchema']>>
                >()
            const spy = vi
                .spyOn(EntityDefinitionsClient.prototype, 'applyEntityDefinitionSchema')
                .mockResolvedValue(expected)

            const result = await client.applyEntityDefinitionSchema('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates getEntityFields', async () => {
            const expected =
                sentinel<Awaited<ReturnType<EntityDefinitionsClient['getEntityFields']>>>()
            const spy = vi
                .spyOn(EntityDefinitionsClient.prototype, 'getEntityFields')
                .mockResolvedValue(expected)

            const result = await client.getEntityFields('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates listEntityDefinitionVersions', async () => {
            const expected =
                sentinel<
                    Awaited<ReturnType<EntityDefinitionsClient['listEntityDefinitionVersions']>>
                >()
            const spy = vi
                .spyOn(EntityDefinitionsClient.prototype, 'listEntityDefinitionVersions')
                .mockResolvedValue(expected)

            const result = await client.listEntityDefinitionVersions('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates getEntityDefinitionVersion', async () => {
            const expected =
                sentinel<
                    Awaited<ReturnType<EntityDefinitionsClient['getEntityDefinitionVersion']>>
                >()
            const spy = vi
                .spyOn(EntityDefinitionsClient.prototype, 'getEntityDefinitionVersion')
                .mockResolvedValue(expected)

            const result = await client.getEntityDefinitionVersion('uuid-1', 2)

            expect(spy).toHaveBeenCalledWith('uuid-1', 2)
            expect(result).toBe(expected)
        })
    })

    describe('API Keys', () => {
        it('delegates getApiKeys', async () => {
            const expected = sentinel<Awaited<ReturnType<ApiKeysClient['getApiKeys']>>>()
            const spy = vi.spyOn(ApiKeysClient.prototype, 'getApiKeys').mockResolvedValue(expected)

            const result = await client.getApiKeys(1, 20)

            expect(spy).toHaveBeenCalledWith(1, 20)
            expect(result).toBe(expected)
        })

        it('delegates createApiKey', async () => {
            const data = arg<Parameters<ApiKeysClient['createApiKey']>[0]>()
            const expected = sentinel<Awaited<ReturnType<ApiKeysClient['createApiKey']>>>()
            const spy = vi
                .spyOn(ApiKeysClient.prototype, 'createApiKey')
                .mockResolvedValue(expected)

            const result = await client.createApiKey(data)

            expect(spy).toHaveBeenCalledWith(data)
            expect(result).toBe(expected)
        })

        it('delegates revokeApiKey', async () => {
            const expected = sentinel<Awaited<ReturnType<ApiKeysClient['revokeApiKey']>>>()
            const spy = vi
                .spyOn(ApiKeysClient.prototype, 'revokeApiKey')
                .mockResolvedValue(expected)

            const result = await client.revokeApiKey('key-1')

            expect(spy).toHaveBeenCalledWith('key-1')
            expect(result).toBe(expected)
        })

        it('delegates reassignApiKey', async () => {
            const data = arg<Parameters<ApiKeysClient['reassignApiKey']>[1]>()
            const expected = sentinel<Awaited<ReturnType<ApiKeysClient['reassignApiKey']>>>()
            const spy = vi
                .spyOn(ApiKeysClient.prototype, 'reassignApiKey')
                .mockResolvedValue(expected)

            const result = await client.reassignApiKey('key-1', data)

            expect(spy).toHaveBeenCalledWith('key-1', data)
            expect(result).toBe(expected)
        })
    })

    describe('Workflows', () => {
        it('delegates listWorkflows', async () => {
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['listWorkflows']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'listWorkflows')
                .mockResolvedValue(expected)

            const result = await client.listWorkflows()

            expect(spy).toHaveBeenCalledWith()
            expect(result).toBe(expected)
        })

        it('delegates getWorkflows', async () => {
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['getWorkflows']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'getWorkflows')
                .mockResolvedValue(expected)

            const result = await client.getWorkflows(1, 20)

            expect(spy).toHaveBeenCalledWith(1, 20)
            expect(result).toBe(expected)
        })

        it('delegates getWorkflow', async () => {
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['getWorkflow']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'getWorkflow')
                .mockResolvedValue(expected)

            const result = await client.getWorkflow('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates createWorkflow', async () => {
            const data = arg<Parameters<WorkflowsClient['createWorkflow']>[0]>()
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['createWorkflow']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'createWorkflow')
                .mockResolvedValue(expected)

            const result = await client.createWorkflow(data)

            expect(spy).toHaveBeenCalledWith(data)
            expect(result).toBe(expected)
        })

        it('delegates updateWorkflow', async () => {
            const data = arg<Parameters<WorkflowsClient['updateWorkflow']>[1]>()
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['updateWorkflow']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'updateWorkflow')
                .mockResolvedValue(expected)

            const result = await client.updateWorkflow('uuid-1', data)

            expect(spy).toHaveBeenCalledWith('uuid-1', data)
            expect(result).toBe(expected)
        })

        it('delegates deleteWorkflow', async () => {
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['deleteWorkflow']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'deleteWorkflow')
                .mockResolvedValue(expected)

            const result = await client.deleteWorkflow('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates runWorkflow', async () => {
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['runWorkflow']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'runWorkflow')
                .mockResolvedValue(expected)

            const result = await client.runWorkflow('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates previewCron', async () => {
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['previewCron']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'previewCron')
                .mockResolvedValue(expected)

            const result = await client.previewCron('* * * * *')

            expect(spy).toHaveBeenCalledWith('* * * * *')
            expect(result).toBe(expected)
        })

        it('delegates getWorkflowRuns', async () => {
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['getWorkflowRuns']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'getWorkflowRuns')
                .mockResolvedValue(expected)

            const result = await client.getWorkflowRuns('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates getWorkflowRunLogs', async () => {
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['getWorkflowRunLogs']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'getWorkflowRunLogs')
                .mockResolvedValue(expected)

            const result = await client.getWorkflowRunLogs('run-1', 2)

            expect(spy).toHaveBeenCalledWith('run-1', 2)
            expect(result).toBe(expected)
        })

        it('delegates getAllWorkflowRuns', async () => {
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['getAllWorkflowRuns']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'getAllWorkflowRuns')
                .mockResolvedValue(expected)

            const result = await client.getAllWorkflowRuns()

            expect(spy).toHaveBeenCalledWith()
            expect(result).toBe(expected)
        })

        it('delegates uploadRunFile', async () => {
            const file = arg<Parameters<WorkflowsClient['uploadRunFile']>[1]>()
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['uploadRunFile']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'uploadRunFile')
                .mockResolvedValue(expected)

            const result = await client.uploadRunFile('uuid-1', file)

            expect(spy).toHaveBeenCalledWith('uuid-1', file)
            expect(result).toBe(expected)
        })

        it('delegates getDslFromOptions', async () => {
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['getDslFromOptions']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'getDslFromOptions')
                .mockResolvedValue(expected)

            const result = await client.getDslFromOptions()

            expect(spy).toHaveBeenCalledWith()
            expect(result).toBe(expected)
        })

        it('delegates getDslToOptions', async () => {
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['getDslToOptions']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'getDslToOptions')
                .mockResolvedValue(expected)

            const result = await client.getDslToOptions()

            expect(spy).toHaveBeenCalledWith()
            expect(result).toBe(expected)
        })

        it('delegates getDslTransformOptions', async () => {
            const expected =
                sentinel<Awaited<ReturnType<WorkflowsClient['getDslTransformOptions']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'getDslTransformOptions')
                .mockResolvedValue(expected)

            const result = await client.getDslTransformOptions()

            expect(spy).toHaveBeenCalledWith()
            expect(result).toBe(expected)
        })

        it('delegates validateDsl', async () => {
            const steps = arg<Parameters<WorkflowsClient['validateDsl']>[0]>()
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['validateDsl']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'validateDsl')
                .mockResolvedValue(expected)

            const result = await client.validateDsl(steps)

            expect(spy).toHaveBeenCalledWith(steps)
            expect(result).toBe(expected)
        })

        it('delegates listWorkflowVersions', async () => {
            const expected =
                sentinel<Awaited<ReturnType<WorkflowsClient['listWorkflowVersions']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'listWorkflowVersions')
                .mockResolvedValue(expected)

            const result = await client.listWorkflowVersions('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates getWorkflowVersion', async () => {
            const expected = sentinel<Awaited<ReturnType<WorkflowsClient['getWorkflowVersion']>>>()
            const spy = vi
                .spyOn(WorkflowsClient.prototype, 'getWorkflowVersion')
                .mockResolvedValue(expected)

            const result = await client.getWorkflowVersion('uuid-1', 2)

            expect(spy).toHaveBeenCalledWith('uuid-1', 2)
            expect(result).toBe(expected)
        })
    })

    describe('Auth', () => {
        it('delegates login', async () => {
            const credentials = arg<Parameters<AuthClient['login']>[0]>()
            const expected = sentinel<Awaited<ReturnType<AuthClient['login']>>>()
            const spy = vi.spyOn(AuthClient.prototype, 'login').mockResolvedValue(expected)

            const result = await client.login(credentials)

            expect(spy).toHaveBeenCalledWith(credentials)
            expect(result).toBe(expected)
        })

        it('delegates refreshToken', async () => {
            const req = arg<Parameters<AuthClient['refreshToken']>[0]>()
            const expected = sentinel<Awaited<ReturnType<AuthClient['refreshToken']>>>()
            const spy = vi.spyOn(AuthClient.prototype, 'refreshToken').mockResolvedValue(expected)

            const result = await client.refreshToken(req)

            expect(spy).toHaveBeenCalledWith(req)
            expect(result).toBe(expected)
        })

        it('delegates logout', async () => {
            const req = arg<Parameters<AuthClient['logout']>[0]>()
            const expected = sentinel<Awaited<ReturnType<AuthClient['logout']>>>()
            const spy = vi.spyOn(AuthClient.prototype, 'logout').mockResolvedValue(expected)

            const result = await client.logout(req)

            expect(spy).toHaveBeenCalledWith(req)
            expect(result).toBe(expected)
        })

        it('delegates revokeAllTokens', async () => {
            const expected = sentinel<Awaited<ReturnType<AuthClient['revokeAllTokens']>>>()
            const spy = vi
                .spyOn(AuthClient.prototype, 'revokeAllTokens')
                .mockResolvedValue(expected)

            const result = await client.revokeAllTokens()

            expect(spy).toHaveBeenCalledWith()
            expect(result).toBe(expected)
        })

        it('delegates getUserPermissions', async () => {
            const expected = sentinel<Awaited<ReturnType<AuthClient['getUserPermissions']>>>()
            const spy = vi
                .spyOn(AuthClient.prototype, 'getUserPermissions')
                .mockResolvedValue(expected)

            const result = await client.getUserPermissions()

            expect(spy).toHaveBeenCalledWith()
            expect(result).toBe(expected)
        })

        it('delegates forgotPassword', async () => {
            const spy = vi
                .spyOn(AuthClient.prototype, 'forgotPassword')
                .mockResolvedValue(undefined)

            await client.forgotPassword('user@example.com')

            expect(spy).toHaveBeenCalledWith('user@example.com')
        })

        it('delegates resetPassword', async () => {
            const spy = vi.spyOn(AuthClient.prototype, 'resetPassword').mockResolvedValue(undefined)

            await client.resetPassword('reset-token', 'new-password')

            expect(spy).toHaveBeenCalledWith('reset-token', 'new-password')
        })
    })

    describe('Users', () => {
        it('delegates getUsers', async () => {
            const expected = sentinel<Awaited<ReturnType<UsersClient['getUsers']>>>()
            const spy = vi.spyOn(UsersClient.prototype, 'getUsers').mockResolvedValue(expected)

            const result = await client.getUsers(1, 20)

            expect(spy).toHaveBeenCalledWith(1, 20)
            expect(result).toBe(expected)
        })

        it('delegates getUser', async () => {
            const expected = sentinel<Awaited<ReturnType<UsersClient['getUser']>>>()
            const spy = vi.spyOn(UsersClient.prototype, 'getUser').mockResolvedValue(expected)

            const result = await client.getUser('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates createUser', async () => {
            const data = arg<Parameters<UsersClient['createUser']>[0]>()
            const expected = sentinel<Awaited<ReturnType<UsersClient['createUser']>>>()
            const spy = vi.spyOn(UsersClient.prototype, 'createUser').mockResolvedValue(expected)

            const result = await client.createUser(data)

            expect(spy).toHaveBeenCalledWith(data)
            expect(result).toBe(expected)
        })

        it('delegates updateUser', async () => {
            const data = arg<Parameters<UsersClient['updateUser']>[1]>()
            const expected = sentinel<Awaited<ReturnType<UsersClient['updateUser']>>>()
            const spy = vi.spyOn(UsersClient.prototype, 'updateUser').mockResolvedValue(expected)

            const result = await client.updateUser('uuid-1', data)

            expect(spy).toHaveBeenCalledWith('uuid-1', data)
            expect(result).toBe(expected)
        })

        it('delegates deleteUser', async () => {
            const expected = sentinel<Awaited<ReturnType<UsersClient['deleteUser']>>>()
            const spy = vi.spyOn(UsersClient.prototype, 'deleteUser').mockResolvedValue(expected)

            const result = await client.deleteUser('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates getUserRoles', async () => {
            const expected = sentinel<Awaited<ReturnType<UsersClient['getUserRoles']>>>()
            const spy = vi.spyOn(UsersClient.prototype, 'getUserRoles').mockResolvedValue(expected)

            const result = await client.getUserRoles('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates assignRolesToUser', async () => {
            const expected = sentinel<Awaited<ReturnType<UsersClient['assignRolesToUser']>>>()
            const spy = vi
                .spyOn(UsersClient.prototype, 'assignRolesToUser')
                .mockResolvedValue(expected)

            const result = await client.assignRolesToUser('uuid-1', ['role-1', 'role-2'])

            expect(spy).toHaveBeenCalledWith('uuid-1', ['role-1', 'role-2'])
            expect(result).toBe(expected)
        })
    })

    describe('Entities', () => {
        it('delegates getEntities', async () => {
            const expected = sentinel<Awaited<ReturnType<EntitiesClient['getEntities']>>>()
            const spy = vi
                .spyOn(EntitiesClient.prototype, 'getEntities')
                .mockResolvedValue(expected)

            const result = await client.getEntities('product', 1, 20)

            expect(spy).toHaveBeenCalledWith('product', 1, 20)
            expect(result).toBe(expected)
        })

        it('delegates browseByPath', async () => {
            const expected = sentinel<Awaited<ReturnType<EntitiesClient['browseByPath']>>>()
            const spy = vi
                .spyOn(EntitiesClient.prototype, 'browseByPath')
                .mockResolvedValue(expected)

            const result = await client.browseByPath('/root')

            expect(spy).toHaveBeenCalledWith('/root')
            expect(result).toBe(expected)
        })

        it('delegates searchEntitiesByPath', async () => {
            const expected = sentinel<Awaited<ReturnType<EntitiesClient['searchEntitiesByPath']>>>()
            const spy = vi
                .spyOn(EntitiesClient.prototype, 'searchEntitiesByPath')
                .mockResolvedValue(expected)

            const result = await client.searchEntitiesByPath('term', 5)

            expect(spy).toHaveBeenCalledWith('term', 5)
            expect(result).toBe(expected)
        })

        it('delegates queryEntities', async () => {
            const expected = sentinel<Awaited<ReturnType<EntitiesClient['queryEntities']>>>()
            const spy = vi
                .spyOn(EntitiesClient.prototype, 'queryEntities')
                .mockResolvedValue(expected)
            const options = arg<Parameters<EntitiesClient['queryEntities']>[1]>()

            const result = await client.queryEntities('product', options)

            expect(spy).toHaveBeenCalledWith('product', options)
            expect(result).toBe(expected)
        })

        it('delegates getEntity', async () => {
            const expected = sentinel<Awaited<ReturnType<EntitiesClient['getEntity']>>>()
            const spy = vi.spyOn(EntitiesClient.prototype, 'getEntity').mockResolvedValue(expected)

            const result = await client.getEntity('product', 'uuid-1')

            expect(spy).toHaveBeenCalledWith('product', 'uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates createEntity', async () => {
            const data = arg<Parameters<EntitiesClient['createEntity']>[1]>()
            const expected = sentinel<Awaited<ReturnType<EntitiesClient['createEntity']>>>()
            const spy = vi
                .spyOn(EntitiesClient.prototype, 'createEntity')
                .mockResolvedValue(expected)

            const result = await client.createEntity('product', data)

            expect(spy).toHaveBeenCalledWith('product', data)
            expect(result).toBe(expected)
        })

        it('delegates updateEntity', async () => {
            const data = arg<Parameters<EntitiesClient['updateEntity']>[2]>()
            const expected = sentinel<Awaited<ReturnType<EntitiesClient['updateEntity']>>>()
            const spy = vi
                .spyOn(EntitiesClient.prototype, 'updateEntity')
                .mockResolvedValue(expected)

            const result = await client.updateEntity('product', 'uuid-1', data)

            expect(spy).toHaveBeenCalledWith('product', 'uuid-1', data)
            expect(result).toBe(expected)
        })

        it('delegates deleteEntity', async () => {
            const expected = sentinel<Awaited<ReturnType<EntitiesClient['deleteEntity']>>>()
            const spy = vi
                .spyOn(EntitiesClient.prototype, 'deleteEntity')
                .mockResolvedValue(expected)

            const result = await client.deleteEntity('product', 'uuid-1')

            expect(spy).toHaveBeenCalledWith('product', 'uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates listEntityVersions', async () => {
            const expected = sentinel<Awaited<ReturnType<EntitiesClient['listEntityVersions']>>>()
            const spy = vi
                .spyOn(EntitiesClient.prototype, 'listEntityVersions')
                .mockResolvedValue(expected)

            const result = await client.listEntityVersions('product', 'uuid-1')

            expect(spy).toHaveBeenCalledWith('product', 'uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates getEntityVersion', async () => {
            const expected = sentinel<Awaited<ReturnType<EntitiesClient['getEntityVersion']>>>()
            const spy = vi
                .spyOn(EntitiesClient.prototype, 'getEntityVersion')
                .mockResolvedValue(expected)

            const result = await client.getEntityVersion('product', 'uuid-1', 2)

            expect(spy).toHaveBeenCalledWith('product', 'uuid-1', 2)
            expect(result).toBe(expected)
        })
    })

    describe('System', () => {
        it('delegates getEntityVersioningSettings', async () => {
            const expected =
                sentinel<Awaited<ReturnType<SystemClient['getEntityVersioningSettings']>>>()
            const spy = vi
                .spyOn(SystemClient.prototype, 'getEntityVersioningSettings')
                .mockResolvedValue(expected)

            const result = await client.getEntityVersioningSettings()

            expect(spy).toHaveBeenCalledWith()
            expect(result).toBe(expected)
        })

        it('delegates updateEntityVersioningSettings', async () => {
            const data = arg<Parameters<SystemClient['updateEntityVersioningSettings']>[0]>()
            const expected =
                sentinel<Awaited<ReturnType<SystemClient['updateEntityVersioningSettings']>>>()
            const spy = vi
                .spyOn(SystemClient.prototype, 'updateEntityVersioningSettings')
                .mockResolvedValue(expected)

            const result = await client.updateEntityVersioningSettings(data)

            expect(spy).toHaveBeenCalledWith(data)
            expect(result).toBe(expected)
        })

        it('delegates getWorkflowRunLogSettings', async () => {
            const expected =
                sentinel<Awaited<ReturnType<SystemClient['getWorkflowRunLogSettings']>>>()
            const spy = vi
                .spyOn(SystemClient.prototype, 'getWorkflowRunLogSettings')
                .mockResolvedValue(expected)

            const result = await client.getWorkflowRunLogSettings()

            expect(spy).toHaveBeenCalledWith()
            expect(result).toBe(expected)
        })

        it('delegates updateWorkflowRunLogSettings', async () => {
            const data = arg<Parameters<SystemClient['updateWorkflowRunLogSettings']>[0]>()
            const expected =
                sentinel<Awaited<ReturnType<SystemClient['updateWorkflowRunLogSettings']>>>()
            const spy = vi
                .spyOn(SystemClient.prototype, 'updateWorkflowRunLogSettings')
                .mockResolvedValue(expected)

            const result = await client.updateWorkflowRunLogSettings(data)

            expect(spy).toHaveBeenCalledWith(data)
            expect(result).toBe(expected)
        })

        it('delegates getLicenseStatus', async () => {
            const expected = sentinel<Awaited<ReturnType<SystemClient['getLicenseStatus']>>>()
            const spy = vi
                .spyOn(SystemClient.prototype, 'getLicenseStatus')
                .mockResolvedValue(expected)

            const result = await client.getLicenseStatus()

            expect(spy).toHaveBeenCalledWith()
            expect(result).toBe(expected)
        })

        it('delegates getSystemVersions', async () => {
            const expected = sentinel<Awaited<ReturnType<SystemClient['getSystemVersions']>>>()
            const spy = vi
                .spyOn(SystemClient.prototype, 'getSystemVersions')
                .mockResolvedValue(expected)

            const result = await client.getSystemVersions()

            expect(spy).toHaveBeenCalledWith()
            expect(result).toBe(expected)
        })
    })

    describe('Roles', () => {
        it('delegates getRoles', async () => {
            const expected = sentinel<Awaited<ReturnType<RolesClient['getRoles']>>>()
            const spy = vi.spyOn(RolesClient.prototype, 'getRoles').mockResolvedValue(expected)

            const result = await client.getRoles(1, 20)

            expect(spy).toHaveBeenCalledWith(1, 20)
            expect(result).toBe(expected)
        })

        it('delegates getRole', async () => {
            const expected = sentinel<Awaited<ReturnType<RolesClient['getRole']>>>()
            const spy = vi.spyOn(RolesClient.prototype, 'getRole').mockResolvedValue(expected)

            const result = await client.getRole('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates createRole', async () => {
            const data = arg<Parameters<RolesClient['createRole']>[0]>()
            const expected = sentinel<Awaited<ReturnType<RolesClient['createRole']>>>()
            const spy = vi.spyOn(RolesClient.prototype, 'createRole').mockResolvedValue(expected)

            const result = await client.createRole(data)

            expect(spy).toHaveBeenCalledWith(data)
            expect(result).toBe(expected)
        })

        it('delegates updateRole', async () => {
            const data = arg<Parameters<RolesClient['updateRole']>[1]>()
            const expected = sentinel<Awaited<ReturnType<RolesClient['updateRole']>>>()
            const spy = vi.spyOn(RolesClient.prototype, 'updateRole').mockResolvedValue(expected)

            const result = await client.updateRole('uuid-1', data)

            expect(spy).toHaveBeenCalledWith('uuid-1', data)
            expect(result).toBe(expected)
        })

        it('delegates deleteRole', async () => {
            const expected = sentinel<Awaited<ReturnType<RolesClient['deleteRole']>>>()
            const spy = vi.spyOn(RolesClient.prototype, 'deleteRole').mockResolvedValue(expected)

            const result = await client.deleteRole('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates assignRolesToApiKey', async () => {
            const expected = sentinel<Awaited<ReturnType<RolesClient['assignRolesToApiKey']>>>()
            const spy = vi
                .spyOn(RolesClient.prototype, 'assignRolesToApiKey')
                .mockResolvedValue(expected)

            const data = arg<Parameters<RolesClient['assignRolesToApiKey']>[1]>()
            const result = await client.assignRolesToApiKey('key-1', data)

            expect(spy).toHaveBeenCalledWith('key-1', data)
            expect(result).toBe(expected)
        })
    })

    describe('Meta', () => {
        it('delegates getDashboardStats', async () => {
            const expected = sentinel<Awaited<ReturnType<MetaClient['getDashboardStats']>>>()
            const spy = vi
                .spyOn(MetaClient.prototype, 'getDashboardStats')
                .mockResolvedValue(expected)

            const result = await client.getDashboardStats()

            expect(spy).toHaveBeenCalledWith()
            expect(result).toBe(expected)
        })
    })

    describe('Capabilities', () => {
        it('delegates getCapabilities', async () => {
            const expected = sentinel<Awaited<ReturnType<CapabilitiesClient['getCapabilities']>>>()
            const spy = vi
                .spyOn(CapabilitiesClient.prototype, 'getCapabilities')
                .mockResolvedValue(expected)

            const result = await client.getCapabilities()

            expect(spy).toHaveBeenCalledWith()
            expect(result).toBe(expected)
        })
    })

    describe('Email Templates', () => {
        it('delegates listEmailTemplates to EmailTemplateClient.list', async () => {
            const expected = sentinel<Awaited<ReturnType<EmailTemplateClient['list']>>>()
            const spy = vi.spyOn(EmailTemplateClient.prototype, 'list').mockResolvedValue(expected)

            const result = await client.listEmailTemplates('system')

            expect(spy).toHaveBeenCalledWith('system')
            expect(result).toBe(expected)
        })

        it('delegates getEmailTemplateByUuid to EmailTemplateClient.getByUuid', async () => {
            const expected = sentinel<Awaited<ReturnType<EmailTemplateClient['getByUuid']>>>()
            const spy = vi
                .spyOn(EmailTemplateClient.prototype, 'getByUuid')
                .mockResolvedValue(expected)

            const result = await client.getEmailTemplateByUuid('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })

        it('delegates createEmailTemplate to EmailTemplateClient.create', async () => {
            const data = arg<Parameters<EmailTemplateClient['create']>[0]>()
            const expected = sentinel<Awaited<ReturnType<EmailTemplateClient['create']>>>()
            const spy = vi
                .spyOn(EmailTemplateClient.prototype, 'create')
                .mockResolvedValue(expected)

            const result = await client.createEmailTemplate(data)

            expect(spy).toHaveBeenCalledWith(data)
            expect(result).toBe(expected)
        })

        it('delegates updateEmailTemplate to EmailTemplateClient.update', async () => {
            const data = arg<Parameters<EmailTemplateClient['update']>[1]>()
            const spy = vi
                .spyOn(EmailTemplateClient.prototype, 'update')
                .mockResolvedValue(undefined)

            await client.updateEmailTemplate('uuid-1', data)

            expect(spy).toHaveBeenCalledWith('uuid-1', data)
        })

        it('delegates deleteEmailTemplate to EmailTemplateClient.delete', async () => {
            const spy = vi
                .spyOn(EmailTemplateClient.prototype, 'delete')
                .mockResolvedValue(undefined)

            await client.deleteEmailTemplate('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
        })
    })

    describe('System Logs', () => {
        it('delegates listSystemLogs to SystemLogClient.list', async () => {
            const params = arg<Parameters<SystemLogClient['list']>[0]>()
            const expected = sentinel<Awaited<ReturnType<SystemLogClient['list']>>>()
            const spy = vi.spyOn(SystemLogClient.prototype, 'list').mockResolvedValue(expected)

            const result = await client.listSystemLogs(params)

            expect(spy).toHaveBeenCalledWith(params)
            expect(result).toBe(expected)
        })

        it('delegates getSystemLogByUuid to SystemLogClient.getByUuid', async () => {
            const expected = sentinel<Awaited<ReturnType<SystemLogClient['getByUuid']>>>()
            const spy = vi.spyOn(SystemLogClient.prototype, 'getByUuid').mockResolvedValue(expected)

            const result = await client.getSystemLogByUuid('uuid-1')

            expect(spy).toHaveBeenCalledWith('uuid-1')
            expect(result).toBe(expected)
        })
    })
})
