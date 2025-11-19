import { computeDiffRows, flattenObject } from './versionDiff'
import { describe, it, expect } from 'vitest'

describe('versionDiff utils', () => {
    it('flattens nested objects', () => {
        const obj = { a: 1, b: { c: 2, d: { e: 'x' } } }
        const flat = flattenObject(obj)
        expect(flat).toEqual({ a: '1', 'b.c': '2', 'b.d.e': 'x' })
    })

    it('computes diff rows', () => {
        const a = { a: 1, b: { c: 2 } }
        const b = { a: 1, b: { c: 3 }, f: true }
        const rows = computeDiffRows(a, b)
        const map = Object.fromEntries(rows.map(r => [r.field, r]))
        expect(map['a'].changed).toBe(false)
        expect(map['b.c'].changed).toBe(true)
        expect(map['f'].changed).toBe(true)
        expect(map['f'].b).toBe('true')
    })
})
