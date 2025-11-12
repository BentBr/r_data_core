import { describe, it, expect } from 'vitest'
import {
    defaultStep,
    defaultCsvOptions,
    sanitizeDslStep,
    ensureCsvOptions,
    type DslStep,
} from './dsl-utils'

describe('dsl-utils', () => {
    describe('defaultStep', () => {
        it('returns a step with format-based structure', () => {
            const step = defaultStep()
            expect(step.from.type).toBe('format')
            expect(step.to.type).toBe('format')

            if (step.from.type === 'format') {
                expect(step.from.source.source_type).toBe('uri')
                expect(step.from.format.format_type).toBe('csv')
            }

            if (step.to.type === 'format') {
                expect(step.to.output.mode).toBe('api')
                expect(step.to.format.format_type).toBe('json')
            }
        })
    })

    describe('defaultCsvOptions', () => {
        it('returns default CSV options with has_header', () => {
            const options = defaultCsvOptions()
            expect(options.has_header).toBe(true)
            expect(options.delimiter).toBe(',')
        })
    })

    describe('sanitizeDslStep', () => {
        it('sanitizes format-based to definition', () => {
            const step: any = {
                from: {
                    type: 'format',
                    source: { source_type: 'uri', config: {} },
                    format: { format_type: 'csv' },
                    mapping: {},
                },
                to: {
                    type: 'format',
                    output: null,
                    format: { format_type: 'json' },
                    mapping: {},
                },
                transform: { type: 'none' },
            }

            const sanitized = sanitizeDslStep(step)
            expect(sanitized.to.type).toBe('format')
            if (sanitized.to.type === 'format') {
                expect(sanitized.to.output.mode).toBe('api')
            }
        })

        it('sanitizes format-based from definition', () => {
            const step: any = {
                from: {
                    type: 'format',
                    source: null,
                    format: null,
                    mapping: {},
                },
                to: {
                    type: 'format',
                    output: { mode: 'api' },
                    format: { format_type: 'json' },
                    mapping: {},
                },
                transform: { type: 'none' },
            }

            const sanitized = sanitizeDslStep(step)
            expect(sanitized.from.type).toBe('format')
            if (sanitized.from.type === 'format') {
                expect(sanitized.from.source.source_type).toBe('uri')
                expect(sanitized.from.format.format_type).toBe('csv')
            }
        })

        it('removes output field from entity type', () => {
            const step: any = {
                from: {
                    type: 'entity',
                    entity_definition: 'test',
                    filter: { field: 'id', value: '1' },
                    mapping: {},
                },
                to: {
                    type: 'entity',
                    output: 'api', // Should be removed
                    entity_definition: 'test',
                    path: '/test',
                    mode: 'create',
                    mapping: {},
                },
                transform: { type: 'none' },
            }

            const sanitized = sanitizeDslStep(step)
            expect(sanitized.to.type).toBe('entity')
            if (sanitized.to.type === 'entity') {
                expect('output' in sanitized.to).toBe(false)
            }
        })
    })

    describe('ensureCsvOptions', () => {
        it('ensures CSV options for format-based from with csv format', () => {
            const step: DslStep = {
                from: {
                    type: 'format',
                    source: { source_type: 'uri', config: {}, auth: { type: 'none' } },
                    format: { format_type: 'csv', options: undefined as any },
                    mapping: {},
                },
                to: {
                    type: 'format',
                    output: { mode: 'api' },
                    format: { format_type: 'json', options: {} },
                    mapping: {},
                },
                transform: { type: 'none' },
            }

            ensureCsvOptions(step)

            if (step.from.type === 'format' && step.from.format?.format_type === 'csv') {
                expect(step.from.format.options).toBeDefined()
                expect(step.from.format.options?.has_header).toBe(true)
            }
        })

        it('ensures CSV options for format-based to with csv format', () => {
            const step: DslStep = {
                from: {
                    type: 'format',
                    source: { source_type: 'uri', config: {}, auth: { type: 'none' } },
                    format: { format_type: 'json', options: {} },
                    mapping: {},
                },
                to: {
                    type: 'format',
                    output: { mode: 'api' },
                    format: { format_type: 'csv', options: undefined as any },
                    mapping: {},
                },
                transform: { type: 'none' },
            }

            ensureCsvOptions(step)

            if (step.to.type === 'format' && step.to.format?.format_type === 'csv') {
                expect(step.to.format.options).toBeDefined()
                expect(step.to.format.options?.has_header).toBe(true)
            }
        })
    })
})
