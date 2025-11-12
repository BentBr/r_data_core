// Main export file for all API clients
// Combines all domain-specific clients into a single TypedHttpClient class

import { BaseTypedHttpClient } from './base'
import { EntityDefinitionsClient } from './entity-definitions'
import { ApiKeysClient } from './api-keys'
import { WorkflowsClient } from './workflows'
import { AuthClient } from './auth'
import { UsersClient } from './users'
import { EntitiesClient } from './entities'

/**
 * Main typed HTTP client that combines all domain-specific clients
 * Provides a unified interface for all API operations
 */
export class TypedHttpClient extends BaseTypedHttpClient {
    // Compose clients as instance properties for efficiency
    private entityDefinitionsClient = new EntityDefinitionsClient()
    private apiKeysClient = new ApiKeysClient()
    private workflowsClient = new WorkflowsClient()
    private authClient = new AuthClient()
    private usersClient = new UsersClient()
    private entitiesClient = new EntitiesClient()

    // Entity Definitions
    async getEntityDefinitions(...args: Parameters<EntityDefinitionsClient['getEntityDefinitions']>) {
        return this.entityDefinitionsClient.getEntityDefinitions(...args)
    }
    async getEntityDefinition(...args: Parameters<EntityDefinitionsClient['getEntityDefinition']>) {
        return this.entityDefinitionsClient.getEntityDefinition(...args)
    }
    async createEntityDefinition(
        ...args: Parameters<EntityDefinitionsClient['createEntityDefinition']>
    ) {
        return this.entityDefinitionsClient.createEntityDefinition(...args)
    }
    async updateEntityDefinition(
        ...args: Parameters<EntityDefinitionsClient['updateEntityDefinition']>
    ) {
        return this.entityDefinitionsClient.updateEntityDefinition(...args)
    }
    async deleteEntityDefinition(
        ...args: Parameters<EntityDefinitionsClient['deleteEntityDefinition']>
    ) {
        return this.entityDefinitionsClient.deleteEntityDefinition(...args)
    }
    async applyEntityDefinitionSchema(
        ...args: Parameters<EntityDefinitionsClient['applyEntityDefinitionSchema']>
    ) {
        return this.entityDefinitionsClient.applyEntityDefinitionSchema(...args)
    }
    async getEntityFields(...args: Parameters<EntityDefinitionsClient['getEntityFields']>) {
        return this.entityDefinitionsClient.getEntityFields(...args)
    }

    // API Keys
    async getApiKeys(...args: Parameters<ApiKeysClient['getApiKeys']>) {
        return this.apiKeysClient.getApiKeys(...args)
    }
    async createApiKey(...args: Parameters<ApiKeysClient['createApiKey']>) {
        return this.apiKeysClient.createApiKey(...args)
    }
    async revokeApiKey(...args: Parameters<ApiKeysClient['revokeApiKey']>) {
        return this.apiKeysClient.revokeApiKey(...args)
    }
    async reassignApiKey(...args: Parameters<ApiKeysClient['reassignApiKey']>) {
        return this.apiKeysClient.reassignApiKey(...args)
    }

    // Workflows
    async listWorkflows(...args: Parameters<WorkflowsClient['listWorkflows']>) {
        return this.workflowsClient.listWorkflows(...args)
    }
    async getWorkflows(...args: Parameters<WorkflowsClient['getWorkflows']>) {
        return this.workflowsClient.getWorkflows(...args)
    }
    async getWorkflow(...args: Parameters<WorkflowsClient['getWorkflow']>) {
        return this.workflowsClient.getWorkflow(...args)
    }
    async createWorkflow(...args: Parameters<WorkflowsClient['createWorkflow']>) {
        return this.workflowsClient.createWorkflow(...args)
    }
    async updateWorkflow(...args: Parameters<WorkflowsClient['updateWorkflow']>) {
        return this.workflowsClient.updateWorkflow(...args)
    }
    async deleteWorkflow(...args: Parameters<WorkflowsClient['deleteWorkflow']>) {
        return this.workflowsClient.deleteWorkflow(...args)
    }
    async runWorkflow(...args: Parameters<WorkflowsClient['runWorkflow']>) {
        return this.workflowsClient.runWorkflow(...args)
    }
    async previewCron(...args: Parameters<WorkflowsClient['previewCron']>) {
        return this.workflowsClient.previewCron(...args)
    }
    async getWorkflowRuns(...args: Parameters<WorkflowsClient['getWorkflowRuns']>) {
        return this.workflowsClient.getWorkflowRuns(...args)
    }
    async getWorkflowRunLogs(...args: Parameters<WorkflowsClient['getWorkflowRunLogs']>) {
        return this.workflowsClient.getWorkflowRunLogs(...args)
    }
    async getAllWorkflowRuns(...args: Parameters<WorkflowsClient['getAllWorkflowRuns']>) {
        return this.workflowsClient.getAllWorkflowRuns(...args)
    }
    async uploadRunFile(...args: Parameters<WorkflowsClient['uploadRunFile']>) {
        return this.workflowsClient.uploadRunFile(...args)
    }
    async getDslFromOptions(...args: Parameters<WorkflowsClient['getDslFromOptions']>) {
        return this.workflowsClient.getDslFromOptions(...args)
    }
    async getDslToOptions(...args: Parameters<WorkflowsClient['getDslToOptions']>) {
        return this.workflowsClient.getDslToOptions(...args)
    }
    async getDslTransformOptions(...args: Parameters<WorkflowsClient['getDslTransformOptions']>) {
        return this.workflowsClient.getDslTransformOptions(...args)
    }
    async validateDsl(...args: Parameters<WorkflowsClient['validateDsl']>) {
        return this.workflowsClient.validateDsl(...args)
    }

    // Auth
    async login(...args: Parameters<AuthClient['login']>) {
        return this.authClient.login(...args)
    }
    async refreshToken(...args: Parameters<AuthClient['refreshToken']>) {
        return this.authClient.refreshToken(...args)
    }
    async logout(...args: Parameters<AuthClient['logout']>) {
        return this.authClient.logout(...args)
    }
    async revokeAllTokens(...args: Parameters<AuthClient['revokeAllTokens']>) {
        return this.authClient.revokeAllTokens(...args)
    }

    // Users
    async getUsers(...args: Parameters<UsersClient['getUsers']>) {
        return this.usersClient.getUsers(...args)
    }

    // Entities
    async getEntities(...args: Parameters<EntitiesClient['getEntities']>) {
        return this.entitiesClient.getEntities(...args)
    }
    async browseByPath(...args: Parameters<EntitiesClient['browseByPath']>) {
        return this.entitiesClient.browseByPath(...args)
    }
    async queryEntities(...args: Parameters<EntitiesClient['queryEntities']>) {
        return this.entitiesClient.queryEntities(...args)
    }
    async getEntity(...args: Parameters<EntitiesClient['getEntity']>) {
        return this.entitiesClient.getEntity(...args)
    }
    async createEntity(...args: Parameters<EntitiesClient['createEntity']>) {
        return this.entitiesClient.createEntity(...args)
    }
    async updateEntity(...args: Parameters<EntitiesClient['updateEntity']>) {
        return this.entitiesClient.updateEntity(...args)
    }
    async deleteEntity(...args: Parameters<EntitiesClient['deleteEntity']>) {
        return this.entitiesClient.deleteEntity(...args)
    }
}

// Export individual clients for direct use if needed
export { BaseTypedHttpClient } from './base'
export { EntityDefinitionsClient } from './entity-definitions'
export { ApiKeysClient } from './api-keys'
export { WorkflowsClient } from './workflows'
export { AuthClient } from './auth'
export { UsersClient } from './users'
export { EntitiesClient } from './entities'

