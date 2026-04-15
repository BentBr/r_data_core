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
            console.log('[E2E Setup] User e2e_viewer_user already exists, skipping')
            return ''
        }
        throw new Error(`Create user failed (${res.status}): ${text}`)
    }
    const body = (await res.json()) as ApiResponse<{ uuid: string }>
    return body.data.uuid
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
