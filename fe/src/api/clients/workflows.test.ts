import { describe, it, expect, vi, beforeEach } from 'vitest'
import { WorkflowsClient } from './workflows'

const mockFetch = vi.fn()
global.fetch = mockFetch

vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        token: 'test-token',
        refreshTokens: vi.fn(),
        logout: vi.fn(),
    }),
}))

vi.mock('@/utils/cookies', () => ({
    getRefreshToken: () => 'refresh-token',
}))

vi.mock('@/env-check', () => ({
    env: {
        apiBaseUrl: 'http://localhost:3000',
        enableApiLogging: false,
        devMode: false,
        defaultPageSize: 10,
    },
    buildApiUrl: (endpoint: string) => `http://localhost:3000${endpoint}`,
}))

// ─── Shared mock data ────────────────────────────────────────────────────────

const mockWorkflowDetail = {
    uuid: 'wf-uuid-1',
    name: 'Test Workflow',
    description: 'A test workflow',
    kind: 'consumer',
    enabled: true,
    schedule_cron: '0 * * * *',
    config: {},
    versioning_disabled: false,
}

const mockWorkflowSummary = {
    uuid: 'wf-uuid-1',
    name: 'Test Workflow',
    kind: 'consumer',
    enabled: true,
    schedule_cron: null,
    has_api_endpoint: false,
    versioning_disabled: false,
}

const mockWorkflowRun = {
    uuid: 'run-uuid-1',
    status: 'completed',
    queued_at: '2024-01-01T10:00:00Z',
    started_at: '2024-01-01T10:00:01Z',
    finished_at: '2024-01-01T10:01:00Z',
    processed_items: 42,
    failed_items: 0,
}

const mockWorkflowRunLog = {
    uuid: 'log-uuid-1',
    ts: '2024-01-01T10:00:30Z',
    level: 'info',
    message: 'Processed item',
    meta: null,
}

const mockPagination = {
    total: 50,
    page: 1,
    per_page: 20,
    total_pages: 3,
    has_previous: false,
    has_next: true,
}

const mockDslOptions = {
    types: [
        {
            type: 'csv',
            fields: [
                { name: 'delimiter', type: 'string', required: false },
                { name: 'has_header', type: 'boolean', required: false },
            ],
        },
    ],
}

const mockWorkflowConfig = {
    steps: [],
}

// ─── Helper to build wrapped API responses ───────────────────────────────────

function successResponse<T>(data: T, meta?: unknown) {
    return { status: 'Success', message: 'OK', data, ...(meta ? { meta } : {}) }
}

function errorResponse(message: string) {
    return { status: 'Error', message }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

describe('WorkflowsClient', () => {
    let client: WorkflowsClient

    beforeEach(() => {
        client = new WorkflowsClient()
        vi.clearAllMocks()
    })

    // ── listWorkflows ──────────────────────────────────────────────────────────

    describe('listWorkflows', () => {
        it('should return an array of workflows', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([mockWorkflowDetail]),
            })

            const result = await client.listWorkflows()

            expect(result).toHaveLength(1)
            expect(result[0].uuid).toBe('wf-uuid-1')
            expect(result[0].name).toBe('Test Workflow')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/workflows'),
                expect.objectContaining({ headers: expect.objectContaining({ Authorization: 'Bearer test-token' }) })
            )
        })

        it('should pass through workflow kind as-is from the backend', async () => {
            const workflows = [
                { ...mockWorkflowDetail, kind: 'consumer' },
                { ...mockWorkflowDetail, uuid: 'wf-uuid-2', kind: 'provider' },
            ]

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse(workflows),
            })

            const result = await client.listWorkflows()

            expect(result[0].kind).toBe('consumer')
            expect(result[1].kind).toBe('provider')
        })

        it('should handle server error', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 500,
                statusText: 'Internal Server Error',
                json: async () => errorResponse('Internal error'),
            })

            await expect(client.listWorkflows()).rejects.toThrow()
        })
    })

    // ── getWorkflows ───────────────────────────────────────────────────────────

    describe('getWorkflows', () => {
        it('should return paginated workflows with default parameters', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([mockWorkflowSummary], { pagination: mockPagination }),
            })

            const result = await client.getWorkflows()

            expect(result.data).toHaveLength(1)
            expect(result.meta?.pagination?.total).toBe(50)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('page=1&per_page=20'),
                expect.any(Object)
            )
        })

        it('should pass custom page and itemsPerPage', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([mockWorkflowSummary], { pagination: mockPagination }),
            })

            await client.getWorkflows(3, 50)

            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('page=3&per_page=50'),
                expect.any(Object)
            )
        })

        it('should append sort parameters when provided', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([mockWorkflowSummary], { pagination: mockPagination }),
            })

            await client.getWorkflows(1, 20, 'name', 'asc')

            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('sort_by=name&sort_order=asc'),
                expect.any(Object)
            )
        })

        it('should omit sort parameters when not provided', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([mockWorkflowSummary]),
            })

            await client.getWorkflows(1, 20)

            const calledUrl: string = mockFetch.mock.calls[0][0] as string
            expect(calledUrl).not.toContain('sort_by')
            expect(calledUrl).not.toContain('sort_order')
        })

        it('should pass through kind as-is in paginated results', async () => {
            const summaryProviderKind = { ...mockWorkflowSummary, kind: 'provider' }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([summaryProviderKind]),
            })

            const result = await client.getWorkflows()

            expect(result.data[0].kind).toBe('provider')
        })
    })

    // ── getWorkflow ────────────────────────────────────────────────────────────

    describe('getWorkflow', () => {
        it('should return a single workflow by uuid', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse(mockWorkflowDetail),
            })

            const result = await client.getWorkflow('wf-uuid-1')

            expect(result.uuid).toBe('wf-uuid-1')
            expect(result.name).toBe('Test Workflow')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/workflows/wf-uuid-1'),
                expect.any(Object)
            )
        })

        it('should pass through consumer kind as-is', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse({ ...mockWorkflowDetail, kind: 'consumer' }),
            })

            const result = await client.getWorkflow('wf-uuid-1')

            expect(result.kind).toBe('consumer')
        })

        it('should pass through provider kind as-is', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse({ ...mockWorkflowDetail, kind: 'provider' }),
            })

            const result = await client.getWorkflow('wf-uuid-1')

            expect(result.kind).toBe('provider')
        })

        it('should throw on 404', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => errorResponse('Workflow not found'),
            })

            await expect(client.getWorkflow('nonexistent')).rejects.toThrow()
        })
    })

    // ── createWorkflow ─────────────────────────────────────────────────────────

    describe('createWorkflow', () => {
        const newWorkflow = {
            name: 'New Workflow',
            description: 'A brand-new workflow',
            kind: 'consumer' as const,
            enabled: true,
            schedule_cron: null,
            config: mockWorkflowConfig,
        }

        it('should create a workflow and return uuid', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse({ uuid: 'new-wf-uuid' }),
            })

            const result = await client.createWorkflow(newWorkflow)

            expect(result.uuid).toBe('new-wf-uuid')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/workflows'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify(newWorkflow),
                })
            )
        })

        it('should throw on validation error (422)', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 422,
                statusText: 'Unprocessable Entity',
                json: async () => ({ status: 'Error', message: 'Validation failed', violations: [{ field: 'name', message: 'Too short', code: 'MIN_LENGTH' }] }),
            })

            await expect(client.createWorkflow(newWorkflow)).rejects.toThrow()
        })

        it('should throw on conflict error (409)', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 409,
                statusText: 'Conflict',
                json: async () => errorResponse('Workflow already exists'),
            })

            await expect(client.createWorkflow(newWorkflow)).rejects.toThrow()
        })
    })

    // ── updateWorkflow ─────────────────────────────────────────────────────────

    describe('updateWorkflow', () => {
        const updatedWorkflow = {
            name: 'Updated Workflow',
            kind: 'consumer' as const,
            enabled: false,
            config: mockWorkflowConfig,
        }

        it('should update a workflow and return message', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse({ message: 'Workflow updated' }),
            })

            const result = await client.updateWorkflow('wf-uuid-1', updatedWorkflow)

            expect(result.message).toBe('Workflow updated')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/workflows/wf-uuid-1'),
                expect.objectContaining({
                    method: 'PUT',
                    body: JSON.stringify(updatedWorkflow),
                })
            )
        })

        it('should throw on 404', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => errorResponse('Workflow not found'),
            })

            await expect(client.updateWorkflow('nonexistent', updatedWorkflow)).rejects.toThrow()
        })
    })

    // ── deleteWorkflow ─────────────────────────────────────────────────────────

    describe('deleteWorkflow', () => {
        it('should delete a workflow and return message', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse({ message: 'Workflow deleted' }),
            })

            const result = await client.deleteWorkflow('wf-uuid-1')

            expect(result.message).toBe('Workflow deleted')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/workflows/wf-uuid-1'),
                expect.objectContaining({ method: 'DELETE' })
            )
        })

        it('should throw on 404 when deleting non-existent workflow', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => errorResponse('Workflow not found'),
            })

            await expect(client.deleteWorkflow('nonexistent')).rejects.toThrow()
        })
    })

    // ── runWorkflow ────────────────────────────────────────────────────────────

    describe('runWorkflow', () => {
        it('should trigger a workflow run and return message', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse({ message: 'Workflow triggered' }),
            })

            const result = await client.runWorkflow('wf-uuid-1')

            expect(result.message).toBe('Workflow triggered')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/workflows/wf-uuid-1/run'),
                expect.objectContaining({ method: 'POST' })
            )
        })

        it('should throw when workflow run fails', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 400,
                statusText: 'Bad Request',
                json: async () => errorResponse('Workflow is disabled'),
            })

            await expect(client.runWorkflow('wf-uuid-1')).rejects.toThrow()
        })
    })

    // ── previewCron ────────────────────────────────────────────────────────────

    describe('previewCron', () => {
        it('should return an array of next cron execution times', async () => {
            const cronTimes = [
                '2024-01-01T01:00:00Z',
                '2024-01-01T02:00:00Z',
                '2024-01-01T03:00:00Z',
            ]

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse(cronTimes),
            })

            const result = await client.previewCron('0 * * * *')

            expect(result).toEqual(cronTimes)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/workflows/cron/preview'),
                expect.any(Object)
            )
        })

        it('should URL-encode the cron expression', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([]),
            })

            await client.previewCron('*/5 * * * *')

            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining(encodeURIComponent('*/5 * * * *')),
                expect.any(Object)
            )
        })

        it('should throw on invalid cron expression', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 400,
                statusText: 'Bad Request',
                json: async () => errorResponse('Invalid cron expression'),
            })

            await expect(client.previewCron('invalid-cron')).rejects.toThrow()
        })
    })

    // ── getWorkflowRuns ────────────────────────────────────────────────────────

    describe('getWorkflowRuns', () => {
        it('should return paginated runs for a workflow', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([mockWorkflowRun], { pagination: mockPagination }),
            })

            const result = await client.getWorkflowRuns('wf-uuid-1')

            expect(result.data).toHaveLength(1)
            expect(result.data[0].uuid).toBe('run-uuid-1')
            expect(result.meta?.pagination?.total).toBe(50)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/workflows/wf-uuid-1/runs'),
                expect.any(Object)
            )
        })

        it('should pass custom page and perPage', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([mockWorkflowRun]),
            })

            await client.getWorkflowRuns('wf-uuid-1', 2, 10)

            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('page=2&per_page=10'),
                expect.any(Object)
            )
        })
    })

    // ── getWorkflowRunLogs ─────────────────────────────────────────────────────

    describe('getWorkflowRunLogs', () => {
        it('should return paginated logs for a workflow run', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([mockWorkflowRunLog], { pagination: mockPagination }),
            })

            const result = await client.getWorkflowRunLogs('run-uuid-1')

            expect(result.data).toHaveLength(1)
            expect(result.data[0].uuid).toBe('log-uuid-1')
            expect(result.data[0].level).toBe('info')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/workflows/runs/run-uuid-1/logs'),
                expect.any(Object)
            )
        })

        it('should use default perPage of 50', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([mockWorkflowRunLog]),
            })

            await client.getWorkflowRunLogs('run-uuid-1')

            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('per_page=50'),
                expect.any(Object)
            )
        })

        it('should pass custom page and perPage', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([]),
            })

            await client.getWorkflowRunLogs('run-uuid-1', 3, 100)

            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('page=3&per_page=100'),
                expect.any(Object)
            )
        })
    })

    // ── getAllWorkflowRuns ─────────────────────────────────────────────────────

    describe('getAllWorkflowRuns', () => {
        it('should return paginated runs across all workflows', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([mockWorkflowRun], { pagination: mockPagination }),
            })

            const result = await client.getAllWorkflowRuns()

            expect(result.data).toHaveLength(1)
            expect(result.meta?.pagination?.has_next).toBe(true)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/workflows/runs'),
                expect.any(Object)
            )
        })

        it('should pass custom pagination parameters', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([]),
            })

            await client.getAllWorkflowRuns(4, 25)

            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('page=4&per_page=25'),
                expect.any(Object)
            )
        })
    })

    // ── uploadRunFile ──────────────────────────────────────────────────────────

    describe('uploadRunFile', () => {
        it('should upload file and return run_uuid and staged_items', async () => {
            const file = new File(['col1,col2\nval1,val2'], 'test.csv', { type: 'text/csv' })

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'File uploaded',
                    data: { run_uuid: 'run-uuid-new', staged_items: 10 },
                }),
            })

            const result = await client.uploadRunFile('wf-uuid-1', file)

            expect(result.run_uuid).toBe('run-uuid-new')
            expect(result.staged_items).toBe(10)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/workflows/wf-uuid-1/run/upload'),
                expect.objectContaining({
                    method: 'POST',
                    headers: expect.objectContaining({ Authorization: 'Bearer test-token' }),
                    body: expect.any(FormData),
                })
            )
        })

        it('should throw when HTTP error occurs during upload', async () => {
            const file = new File(['data'], 'test.csv', { type: 'text/csv' })

            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 413,
                statusText: 'Payload Too Large',
                json: async () => ({ message: 'File too large' }),
            })

            await expect(client.uploadRunFile('wf-uuid-1', file)).rejects.toThrow(
                'HTTP 413: Payload Too Large'
            )
        })

        it('should throw with status message when error body cannot be parsed', async () => {
            const file = new File(['data'], 'test.csv', { type: 'text/csv' })

            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 500,
                statusText: 'Internal Server Error',
                json: async () => {
                    throw new SyntaxError('Not JSON')
                },
            })

            await expect(client.uploadRunFile('wf-uuid-1', file)).rejects.toThrow('HTTP 500: Internal Server Error')
        })

        it('should throw when response data is missing', async () => {
            const file = new File(['data'], 'test.csv', { type: 'text/csv' })

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({
                    status: 'Success',
                    message: 'OK',
                    data: null,
                }),
            })

            await expect(client.uploadRunFile('wf-uuid-1', file)).rejects.toThrow('No data in upload response')
        })
    })

    // ── getDslFromOptions ──────────────────────────────────────────────────────

    describe('getDslFromOptions', () => {
        it('should return DSL from options', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse(mockDslOptions),
            })

            const result = await client.getDslFromOptions()

            expect(result.types).toHaveLength(1)
            expect(result.types[0].type).toBe('csv')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/dsl/from/options'),
                expect.any(Object)
            )
        })
    })

    // ── getDslToOptions ────────────────────────────────────────────────────────

    describe('getDslToOptions', () => {
        it('should return DSL to options', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse(mockDslOptions),
            })

            const result = await client.getDslToOptions()

            expect(result.types).toHaveLength(1)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/dsl/to/options'),
                expect.any(Object)
            )
        })
    })

    // ── getDslTransformOptions ─────────────────────────────────────────────────

    describe('getDslTransformOptions', () => {
        it('should return DSL transform options', async () => {
            const transformOptions = {
                types: [
                    { type: 'arithmetic', fields: [{ name: 'op', type: 'string', required: true }] },
                    { type: 'concat', fields: [{ name: 'separator', type: 'string', required: false }] },
                ],
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse(transformOptions),
            })

            const result = await client.getDslTransformOptions()

            expect(result.types).toHaveLength(2)
            expect(result.types[0].type).toBe('arithmetic')
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/dsl/transform/options'),
                expect.any(Object)
            )
        })
    })

    // ── validateDsl ────────────────────────────────────────────────────────────

    describe('validateDsl', () => {
        const dslSteps = [
            {
                from: {
                    type: 'format' as const,
                    source: {
                        source_type: 'uri',
                        config: { uri: 'https://example.com/data.csv' },
                    },
                    format: { format_type: 'csv' },
                    mapping: { name: 'full_name' },
                },
                to: {
                    type: 'entity' as const,
                    entity_definition: 'person',
                    mode: 'create' as const,
                    mapping: { full_name: 'name' },
                },
                transform: { type: 'none' as const },
            },
        ]

        it('should return valid: true for a valid DSL', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse({ valid: true }),
            })

            const result = await client.validateDsl(dslSteps)

            expect(result.valid).toBe(true)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/dsl/validate'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ steps: dslSteps }),
                })
            )
        })

        it('should return valid: false for an invalid DSL', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse({ valid: false }),
            })

            const result = await client.validateDsl(dslSteps)

            expect(result.valid).toBe(false)
        })

        it('should throw on server-side validation failure', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 400,
                statusText: 'Bad Request',
                json: async () => errorResponse('Invalid DSL structure'),
            })

            await expect(client.validateDsl(dslSteps)).rejects.toThrow()
        })
    })

    // ── listWorkflowVersions ───────────────────────────────────────────────────

    describe('listWorkflowVersions', () => {
        it('should return a list of versions for a workflow', async () => {
            const versions = [
                { version_number: 2, created_at: '2024-02-01T00:00:00Z', created_by: 'user-uuid-1', created_by_name: 'Alice' },
                { version_number: 1, created_at: '2024-01-01T00:00:00Z', created_by: 'user-uuid-1', created_by_name: 'Alice' },
            ]

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse(versions),
            })

            const result = await client.listWorkflowVersions('wf-uuid-1')

            expect(result).toHaveLength(2)
            expect(result[0].version_number).toBe(2)
            expect(result[1].version_number).toBe(1)
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/workflows/wf-uuid-1/versions'),
                expect.any(Object)
            )
        })

        it('should return empty array when no versions exist', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse([]),
            })

            const result = await client.listWorkflowVersions('wf-uuid-1')

            expect(result).toHaveLength(0)
        })
    })

    // ── getWorkflowVersion ─────────────────────────────────────────────────────

    describe('getWorkflowVersion', () => {
        it('should return a specific version snapshot', async () => {
            const versionSnapshot = {
                version_number: 1,
                created_at: '2024-01-01T00:00:00Z',
                created_by: 'user-uuid-1',
                data: { name: 'Old Name', kind: 'consumer', enabled: true, config: {} },
            }

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => successResponse(versionSnapshot),
            })

            const result = await client.getWorkflowVersion('wf-uuid-1', 1)

            expect(result.version_number).toBe(1)
            expect(result.data).toBeDefined()
            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/admin/api/v1/workflows/wf-uuid-1/versions/1'),
                expect.any(Object)
            )
        })

        it('should throw on 404 for non-existent version', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: false,
                status: 404,
                statusText: 'Not Found',
                json: async () => errorResponse('Version not found'),
            })

            await expect(client.getWorkflowVersion('wf-uuid-1', 999)).rejects.toThrow()
        })
    })
})
