const API_BASE_URL = process.env.E2E_API_BASE_URL ?? 'http://rdatacore.docker'
const ADMIN_USERNAME = process.env.E2E_ADMIN_USERNAME ?? 'admin'
const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD ?? 'adminadmin'

// All API responses are wrapped: { status, message, data: T, meta }
interface ApiResponse<T> {
    status: string
    message: string
    data: T
}

export interface TestDataIds {
    entityDefinitionUuid?: string
    roleUuid?: string
    userUuid?: string
    workflowUuid?: string
    apiKeyUuid?: string
}

async function apiRequest(
    method: string,
    path: string,
    token: string,
    body?: Record<string, unknown>
): Promise<Response> {
    const url = `${API_BASE_URL}/admin/api/v1${path}`
    const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${token}`,
    }
    return fetch(url, {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
    })
}

export async function login(
    username: string = ADMIN_USERNAME,
    password: string = ADMIN_PASSWORD
): Promise<string> {
    const url = `${API_BASE_URL}/admin/api/v1/auth/login`
    const res = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username, password }),
    })
    if (!res.ok) {
        throw new Error(`Login failed (${res.status}): ${await res.text()}`)
    }
    const body = (await res.json()) as ApiResponse<{ access_token: string }>
    return body.data.access_token
}

export async function createEntityDefinition(token: string): Promise<string> {
    const res = await apiRequest('POST', '/entity-definitions', token, {
        entity_type: 'e2e_test_product',
        display_name: 'E2E Test Product',
        published: true,
        allow_children: false,
        fields: [
            {
                name: 'title',
                display_name: 'Title',
                field_type: 'Text',
                required: true,
                indexed: false,
                filterable: true,
                unique: false,
            },
            {
                name: 'price',
                display_name: 'Price',
                field_type: 'Float',
                required: false,
                indexed: false,
                filterable: true,
                unique: false,
            },
        ],
    })
    if (!res.ok) {
        const text = await res.text()
        if (res.status === 409) {
            console.log('[E2E Setup] Entity definition e2e_test_product already exists, skipping')
            return ''
        }
        throw new Error(`Create entity definition failed (${res.status}): ${text}`)
    }
    const body = (await res.json()) as ApiResponse<{ uuid: string }>
    return body.data.uuid
}

export async function createRole(token: string): Promise<string> {
    const res = await apiRequest('POST', '/roles', token, {
        name: 'e2e_viewer',
        description: 'E2E test role with read-only permissions',
        permissions: [
            {
                resource_type: 'DashboardStats',
                permission_type: 'Read',
                access_level: 'All',
                resource_uuids: [],
            },
            {
                resource_type: 'EntityDefinitions',
                permission_type: 'Read',
                access_level: 'All',
                resource_uuids: [],
            },
            {
                resource_type: 'Entities',
                permission_type: 'Read',
                access_level: 'All',
                resource_uuids: [],
            },
        ],
    })
    if (!res.ok) {
        const text = await res.text()
        if (res.status === 409) {
            console.log('[E2E Setup] Role e2e_viewer already exists, skipping')
            return ''
        }
        throw new Error(`Create role failed (${res.status}): ${text}`)
    }
    const body = (await res.json()) as ApiResponse<{ uuid: string }>
    return body.data.uuid
}

export async function createUser(token: string, roleUuid: string): Promise<string> {
    const res = await apiRequest('POST', '/users', token, {
        username: 'e2e_viewer_user',
        email: 'e2e_viewer@test.local',
        password: 'e2e_viewer_password_123',
        first_name: 'E2E',
        last_name: 'Viewer',
        role_uuids: roleUuid ? [roleUuid] : [],
    })
    if (!res.ok) {
        const text = await res.text()
        if (res.status === 409) {
            // User exists (possibly soft-deleted). Find it and update its roles
            // so it references the freshly created role UUID.
            return await ensureViewerUserRoles(token, roleUuid)
        }
        throw new Error(`Create user failed (${res.status}): ${text}`)
    }
    const body = (await res.json()) as ApiResponse<{ uuid: string }>
    return body.data.uuid
}

async function ensureViewerUserRoles(token: string, roleUuid: string): Promise<string> {
    const listRes = await apiRequest('GET', '/users?per_page=100', token)
    if (!listRes.ok) {
        throw new Error(`Failed to list users for viewer role update`)
    }

    const body = (await listRes.json()) as ApiResponse<
        Array<{ uuid: string; username?: string; is_active?: boolean }>
    >
    const user = body.data.find(u => u.username === 'e2e_viewer_user')
    if (!user) {
        throw new Error('Viewer user exists (409) but not found in user list')
    }

    // Update user: ensure active and correct role assignment
    const updateRes = await apiRequest('PUT', `/users/${user.uuid}`, token, {
        username: 'e2e_viewer_user',
        email: 'e2e_viewer@test.local',
        password: 'e2e_viewer_password_123',
        first_name: 'E2E',
        last_name: 'Viewer',
        is_active: true,
        role_uuids: roleUuid ? [roleUuid] : [],
    })
    if (!updateRes.ok) {
        const text = await updateRes.text()
        throw new Error(`Failed to update viewer user roles (${updateRes.status}): ${text}`)
    }

    console.log(`[E2E Setup] Updated existing viewer user ${user.uuid} with fresh role`)
    return user.uuid
}

export async function createWorkflow(token: string): Promise<string> {
    const res = await apiRequest('POST', '/workflows', token, {
        name: 'e2e_test_workflow',
        kind: 'consumer',
        enabled: false,
        config: {
            steps: [
                {
                    from: {
                        type: 'format',
                        source: { source_type: 'api', config: {} },
                        format: { format_type: 'json', options: {} },
                        mapping: { title: 'title' },
                    },
                    transform: { type: 'none' },
                    to: {
                        type: 'format',
                        output: { mode: 'api' },
                        format: { format_type: 'json', options: {} },
                        mapping: { title: 'title' },
                    },
                },
            ],
        },
    })
    if (!res.ok) {
        const text = await res.text()
        if (res.status === 409) {
            console.log('[E2E Setup] Workflow e2e_test_workflow already exists, skipping')
            return ''
        }
        throw new Error(`Create workflow failed (${res.status}): ${text}`)
    }
    const body = (await res.json()) as ApiResponse<{ uuid: string }>
    return body.data.uuid
}

export async function createApiKey(token: string): Promise<string> {
    const res = await apiRequest('POST', '/api-keys', token, {
        name: 'e2e_test_key',
        description: 'E2E test API key',
    })
    if (!res.ok) {
        const text = await res.text()
        if (res.status === 409) {
            console.log('[E2E Setup] API key e2e_test_key already exists, skipping')
            return ''
        }
        throw new Error(`Create API key failed (${res.status}): ${text}`)
    }
    const body = (await res.json()) as ApiResponse<{ uuid: string }>
    return body.data.uuid
}

/**
 * Delete a user by username. Searches all pages to find the user.
 * Returns true if deleted, false if not found.
 */
export async function deleteUserByUsername(token: string, username: string): Promise<boolean> {
    const res = await apiRequest('GET', `/users?per_page=100`, token)
    if (!res.ok) return false

    const body = (await res.json()) as ApiResponse<Array<{ uuid: string; username?: string }>>
    const user = body.data.find(u => u.username === username)
    if (!user) return false

    const delRes = await apiRequest('DELETE', `/users/${user.uuid}`, token)
    if (delRes.ok) {
        console.log(`[E2E Setup] Deleted stale user ${user.uuid} (${username})`)
    }
    return delRes.ok
}

export async function deleteByPrefix(
    token: string,
    endpoint: string,
    prefix: string
): Promise<void> {
    const res = await apiRequest('GET', `/${endpoint}`, token)
    if (!res.ok) return

    const body = (await res.json()) as ApiResponse<
        Array<{ uuid: string; name?: string; username?: string; entity_type?: string }>
    >
    const items = body.data

    for (const item of items) {
        const name = item.name ?? item.username ?? item.entity_type ?? ''
        if (name.startsWith(prefix)) {
            await apiRequest('DELETE', `/${endpoint}/${item.uuid}`, token)
            console.log(`[E2E Teardown] Deleted ${endpoint}/${item.uuid} (${name})`)
        }
    }
}
