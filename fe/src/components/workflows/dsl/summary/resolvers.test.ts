import { describe, it, expect } from 'vitest'
import { createTypeResolver, createKeyResolver, createPartialKeyResolver } from './resolvers'
import type { TranslateFn } from './types'

const t: TranslateFn = key => key

describe('createTypeResolver', () => {
    const handlers = {
        foo: (value: { type: string; x: number }, _t: TranslateFn) => `foo:${value.x}`,
        bar: (value: { type: string; y: string }, _t: TranslateFn) => `bar:${value.y}`,
    }

    type TestMap = {
        foo: { type: string; x: number }
        bar: { type: string; y: string }
    }

    const resolver = createTypeResolver<TestMap>(handlers)

    it('dispatches to the correct handler based on type', () => {
        expect(resolver({ type: 'foo', x: 42 }, t)).toBe('foo:42')
        expect(resolver({ type: 'bar', y: 'hello' }, t)).toBe('bar:hello')
    })

    it('returns the type string when no handler matches', () => {
        const result = resolver({ type: 'unknown' } as never, t)
        expect(result).toBe('unknown')
    })
})

describe('createKeyResolver', () => {
    const handlers = {
        a: (value: { n: number }, _t: TranslateFn) => `a:${value.n}`,
        b: (value: { n: number }, _t: TranslateFn) => `b:${value.n}`,
    }
    const resolver = createKeyResolver<'a' | 'b', { n: number }>(handlers)

    it('dispatches based on the provided key', () => {
        expect(resolver('a', { n: 1 }, t)).toBe('a:1')
        expect(resolver('b', { n: 2 }, t)).toBe('b:2')
    })
})

describe('createPartialKeyResolver', () => {
    const handlers: Partial<Record<'x' | 'y', (v: { val: string }, t: TranslateFn) => string>> = {
        x: (value, _t) => `x:${value.val}`,
    }
    const resolver = createPartialKeyResolver<'x' | 'y', { val: string }>(handlers)

    it('returns the result when a handler exists', () => {
        expect(resolver('x', { val: 'ok' }, t)).toBe('x:ok')
    })

    it('returns undefined when no handler exists for the key', () => {
        expect(resolver('y', { val: 'nope' }, t)).toBeUndefined()
    })
})
