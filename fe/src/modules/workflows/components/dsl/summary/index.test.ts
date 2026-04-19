import { describe, it, expect } from 'vitest'
import type { DslStep, FromDef, ToDef, Transform } from './types'
import type { TranslateFn } from './types'
import {
    describeFrom,
    describeTransform,
    describeTo,
    buildStepSummary,
    getStepStats,
} from './index'

/**
 * Stub translator: returns the key followed by param values so assertions can
 * verify both the key chosen and the parameters passed.
 */
const t: TranslateFn = (key, params) => {
    if (!params || typeof params === 'string') {
        return key
    }
    const parts = Object.entries(params).map(([k, v]) => `${k}=${v}`)
    return `${key}[${parts.join(',')}]`
}

// ── Fixtures ─────────────────────────────────────────────────────────

function formatFrom(sourceType: string, formatType = 'csv', uri = ''): FromDef {
    return {
        type: 'format',
        source: {
            source_type: sourceType,
            config: uri ? { uri } : {},
            auth: { type: 'none' },
        },
        format: { format_type: formatType, options: {} },
        mapping: {},
    }
}

function entityFrom(name = ''): FromDef {
    return {
        type: 'entity',
        entity_definition: name,
        mapping: {},
    }
}

function formatTo(mode: 'api' | 'download' | 'push', formatType = 'json'): ToDef {
    if (mode === 'push') {
        return {
            type: 'format',
            output: {
                mode: 'push',
                destination: {
                    destination_type: 'uri',
                    config: { uri: 'https://example.com/hook' },
                    auth: { type: 'none' },
                },
            },
            format: { format_type: formatType, options: {} },
            mapping: {},
        }
    }
    return {
        type: 'format',
        output: { mode },
        format: { format_type: formatType, options: {} },
        mapping: {},
    }
}

function entityTo(name = ''): ToDef {
    return {
        type: 'entity',
        entity_definition: name,
        path: '/',
        mode: 'create',
        mapping: { a: 'b' },
    }
}

function nextStepTo(): ToDef {
    return { type: 'next_step', mapping: { x: 'y' } }
}

function step(from: FromDef, transform: Transform, to: ToDef): DslStep {
    return { from, transform, to }
}

// ── describeFrom ─────────────────────────────────────────────────────

describe('describeFrom', () => {
    it('describes format/api source', () => {
        const result = describeFrom(formatFrom('api', 'json'), t)
        expect(result).toContain('workflows.dsl.summary.from.api')
        expect(result).toContain('JSON')
    })

    it('describes format/uri source with a URI', () => {
        const result = describeFrom(formatFrom('uri', 'csv', 'https://data.example.com'), t)
        expect(result).toContain('workflows.dsl.summary.from.uri')
        expect(result).toContain('CSV')
        expect(result).toContain('https://data.example.com')
    })

    it('describes format/uri source with empty URI using placeholder', () => {
        const result = describeFrom(formatFrom('uri', 'csv', ''), t)
        expect(result).toContain('workflows.dsl.summary.from.uri')
    })

    it('falls back to generic summary for unknown source types', () => {
        const result = describeFrom(formatFrom('sftp', 'xml'), t)
        expect(result).toContain('workflows.dsl.summary.from.generic')
        expect(result).toContain('XML')
    })

    it('describes entity source with a name', () => {
        const result = describeFrom(entityFrom('Products'), t)
        expect(result).toContain('workflows.dsl.summary.from.entity_named')
        expect(result).toContain('Products')
    })

    it('describes entity source without a name', () => {
        const result = describeFrom(entityFrom(''), t)
        expect(result).toContain('workflows.dsl.summary.from.entity')
    })

    it('describes previous_step source', () => {
        const from: FromDef = { type: 'previous_step', mapping: {} }
        expect(describeFrom(from, t)).toContain('workflows.dsl.summary.from.previous_step')
    })

    it('describes trigger source', () => {
        const from: FromDef = { type: 'trigger', mapping: {} }
        expect(describeFrom(from, t)).toContain('workflows.dsl.summary.from.trigger')
    })
})

// ── describeTransform ────────────────────────────────────────────────

describe('describeTransform', () => {
    it('describes none transform', () => {
        expect(describeTransform({ type: 'none' }, t)).toContain(
            'workflows.dsl.summary.transform.none'
        )
    })

    it('describes arithmetic transform with target', () => {
        const transform: Transform = {
            type: 'arithmetic',
            target: 'total',
            left: { kind: 'field', field: 'price' },
            op: 'mul',
            right: { kind: 'const', value: 1.19 },
        }
        const result = describeTransform(transform, t)
        expect(result).toContain('total')
    })

    it('describes arithmetic transform without target using placeholder', () => {
        const transform: Transform = {
            type: 'arithmetic',
            target: '',
            left: { kind: 'field', field: 'a' },
            op: 'add',
            right: { kind: 'const', value: 1 },
        }
        const result = describeTransform(transform, t)
        expect(result).toContain('workflows.dsl.summary.transform.arithmetic')
    })

    it('describes concat transform', () => {
        const transform: Transform = {
            type: 'concat',
            target: 'full_name',
            left: { kind: 'field', field: 'first' },
            right: { kind: 'field', field: 'last' },
        }
        const result = describeTransform(transform, t)
        expect(result).toContain('full_name')
    })

    it('describes build_path transform', () => {
        const transform: Transform = {
            type: 'build_path',
            target: 'path',
            template: '{category}/{name}',
        }
        const result = describeTransform(transform, t)
        expect(result).toContain('path')
    })

    it('describes resolve_entity_path transform', () => {
        const transform: Transform = {
            type: 'resolve_entity_path',
            target_path: 'parent_path',
            entity_type: 'Category',
            filters: {},
        }
        const result = describeTransform(transform, t)
        expect(result).toContain('Category')
    })

    it('describes get_or_create_entity transform', () => {
        const transform: Transform = {
            type: 'get_or_create_entity',
            target_path: 'entity_path',
            entity_type: 'Brand',
            path_template: '/brands/{name}',
        }
        const result = describeTransform(transform, t)
        expect(result).toContain('Brand')
    })

    it('describes authenticate transform', () => {
        const transform: Transform = {
            type: 'authenticate',
            entity_type: 'User',
            identifier_field: 'email',
            password_field: 'password_hash',
            input_identifier: 'username',
            input_password: 'password',
            target_token: 'jwt_token',
        }
        const result = describeTransform(transform, t)
        expect(result).toContain('User')
    })

    it('gracefully handles unknown transform types', () => {
        const transform = { type: 'unknown_future_type' } as unknown as Transform
        const result = describeTransform(transform, t)
        expect(result).toBe('unknown_future_type')
    })
})

// ── describeTo ───────────────────────────────────────────────────────

describe('describeTo', () => {
    it('describes format/api output', () => {
        const result = describeTo(formatTo('api', 'json'), t)
        expect(result).toContain('workflows.dsl.summary.to.api')
        expect(result).toContain('JSON')
    })

    it('describes format/download output', () => {
        const result = describeTo(formatTo('download', 'csv'), t)
        expect(result).toContain('workflows.dsl.summary.to.download')
        expect(result).toContain('CSV')
    })

    it('describes format/push output with URI', () => {
        const result = describeTo(formatTo('push', 'json'), t)
        expect(result).toContain('workflows.dsl.summary.to.push')
        expect(result).toContain('https://example.com/hook')
    })

    it('describes entity output with name', () => {
        const result = describeTo(entityTo('Products'), t)
        expect(result).toContain('workflows.dsl.summary.to.entity_named')
        expect(result).toContain('Products')
    })

    it('describes entity output without name', () => {
        const result = describeTo(entityTo(''), t)
        expect(result).toContain('workflows.dsl.summary.to.entity')
    })

    it('describes next_step output', () => {
        expect(describeTo(nextStepTo(), t)).toContain('workflows.dsl.summary.to.next_step')
    })
})

// ── buildStepSummary ─────────────────────────────────────────────────

describe('buildStepSummary', () => {
    it('composes from + transform + to into a step summary', () => {
        const s = step(formatFrom('api', 'json'), { type: 'none' }, entityTo('Products'))
        const result = buildStepSummary(s, t)
        expect(result).toContain('workflows.dsl.summary.step')
    })

    it('does not throw for any standard from/transform/to combination', () => {
        const froms: FromDef[] = [
            formatFrom('api', 'json'),
            formatFrom('uri', 'csv', 'https://x.com'),
            entityFrom('Foo'),
            { type: 'previous_step', mapping: {} },
            { type: 'trigger', mapping: {} },
        ]
        const transforms: Transform[] = [
            { type: 'none' },
            {
                type: 'arithmetic',
                target: 't',
                left: { kind: 'field', field: 'a' },
                op: 'add',
                right: { kind: 'const', value: 1 },
            },
        ]
        const tos: ToDef[] = [
            formatTo('api'),
            formatTo('download'),
            formatTo('push'),
            entityTo('Bar'),
            nextStepTo(),
        ]
        for (const f of froms) {
            for (const tr of transforms) {
                for (const to of tos) {
                    expect(() => buildStepSummary(step(f, tr, to), t)).not.toThrow()
                }
            }
        }
    })
})

// ── getStepStats ─────────────────────────────────────────────────────

describe('getStepStats', () => {
    it('returns input and output mapping counts', () => {
        const s = step(
            { ...formatFrom('api'), mapping: { a: 'b', c: 'd' } },
            { type: 'none' },
            { ...formatTo('api'), mapping: { x: 'y' } }
        )
        const stats = getStepStats(s, t)
        expect(stats).toHaveLength(2)
        expect(stats[0].value).toBe('2')
        expect(stats[1].value).toBe('1')
    })

    it('includes transform stat when type is not none', () => {
        const s = step(
            formatFrom('api'),
            {
                type: 'arithmetic',
                target: 'total',
                left: { kind: 'field', field: 'a' },
                op: 'add',
                right: { kind: 'const', value: 1 },
            },
            formatTo('api')
        )
        const stats = getStepStats(s, t)
        expect(stats).toHaveLength(3)
        expect(stats[2].label).toContain('workflows.dsl.stats.transform')
    })

    it('excludes transform stat when type is none', () => {
        const s = step(formatFrom('api'), { type: 'none' }, formatTo('api'))
        const stats = getStepStats(s, t)
        expect(stats).toHaveLength(2)
    })

    it('filters out empty mapping pairs from count', () => {
        const s = step(
            { ...formatFrom('api'), mapping: { '': '', a: 'b' } },
            { type: 'none' },
            formatTo('api')
        )
        const stats = getStepStats(s, t)
        expect(stats[0].value).toBe('1')
    })
})
