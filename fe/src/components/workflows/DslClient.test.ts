import { beforeEach, describe, expect, it, vi } from 'vitest'
import { typedHttpClient } from '@/api/typed-client'
import { vi } from 'vitest'

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
        from: { type: 'csv', uri: 'http://example.com/c.csv', mapping: { price: 'price' } },
        transform: {
          type: 'arithmetic',
          target: 'price',
          left: { kind: 'field', field: 'price' },
          op: 'add',
          right: { kind: 'const', value: 5.0 },
        },
        to: { type: 'json', output: 'api', mapping: { price: 'entity.total' } },
      },
    ]
    const fetchSpy = vi
      .spyOn(global, 'fetch')
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ status: 'Success', message: 'ok', data: { valid: true } }), {
          status: 200,
          headers: { 'Content-Type': 'application/json' },
        }) as any,
      )
    const res = await typedHttpClient.validateDsl(steps)
    expect(res.valid).toBe(true)
    expect(fetchSpy).toHaveBeenCalled()
  })

  it('returns validation error on invalid DSL', async () => {
    const steps = [
      {
        from: { type: 'csv', uri: '', mapping: {} }, // invalid mapping, empty uri
        transform: {
          type: 'arithmetic',
          target: 'x',
          left: { kind: 'const', value: 1 },
          op: 'add',
          right: { kind: 'const', value: 2 },
        },
        to: { type: 'json', output: 'api', mapping: {} },
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
      }) as any,
    )
    await expect(typedHttpClient.validateDsl(steps)).rejects.toMatchObject({
      violations: body.violations,
    })
  })
})


