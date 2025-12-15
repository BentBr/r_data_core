import { beforeEach, describe, expect, it, vi } from 'vitest'
import { typedHttpClient } from '@/api/typed-client'
import type { DslStep } from '@/types/schemas'

vi.mock('@/stores/auth', () => {
    return {
        useAuthStore: () => ({
            token: null,
            refreshTokens: vi.fn().mockResolvedValue(undefined),
            logout: vi.fn().mockResolvedValue(undefined),
        }),
    }
})

describe('DSL client', () => {
    beforeEach(() => {
        vi.restoreAllMocks()
    })

    it('validates a proper DSL via backend', async () => {
        const steps = [
            {
                from: {
                    type: 'format',
                    source: {
                        source_type: 'uri',
                        config: { uri: 'http://example.com/c.csv' },
                    },
                    format: {
                        format_type: 'csv',
                    },
                    mapping: { price: 'price' },
                },
                transform: {
                    type: 'arithmetic',
                    target: 'price',
                    left: { kind: 'field', field: 'price' },
                    op: 'add',
                    right: { kind: 'const', value: 5.0 },
                },
                to: {
                    type: 'format',
                    output: { mode: 'api' },
                    format: { format_type: 'json' },
                    mapping: { price: 'entity.total' },
                },
            },
        ]
        const fetchSpy = vi.spyOn(global, 'fetch').mockResolvedValueOnce(
            new Response(
                JSON.stringify({ status: 'Success', message: 'ok', data: { valid: true } }),
                {
                    status: 200,
                    headers: { 'Content-Type': 'application/json' },
                }
            ) as Response
        )
        const res = await typedHttpClient.validateDsl(steps as unknown as DslStep[])
        expect(res.valid).toBe(true)
        expect(fetchSpy).toHaveBeenCalled()
    })

    it('returns validation error on invalid DSL', async () => {
        const steps = [
            {
                from: {
                    type: 'format',
                    source: {
                        source_type: 'uri',
                        config: { uri: '' },
                    },
                    format: {
                        format_type: 'csv',
                    },
                    mapping: {},
                }, // invalid mapping, empty uri
                transform: {
                    type: 'arithmetic',
                    target: 'x',
                    left: { kind: 'const', value: 1 },
                    op: 'add',
                    right: { kind: 'const', value: 2 },
                },
                to: {
                    type: 'format',
                    output: { mode: 'api' },
                    format: { format_type: 'json' },
                    mapping: {},
                },
            },
        ]
        // Backend responds with 422 Symfony-style
        const body = {
            message: 'Invalid DSL',
            violations: [{ field: 'dsl', message: 'mapping must contain at least one field' }],
        }
        vi.spyOn(global, 'fetch').mockResolvedValueOnce(
            new Response(JSON.stringify(body), {
                status: 422,
                headers: { 'Content-Type': 'application/json' },
            }) as Response
        )
        await expect(
            typedHttpClient.validateDsl(steps as unknown as DslStep[])
        ).rejects.toMatchObject({
            violations: body.violations,
        })
    })
})
