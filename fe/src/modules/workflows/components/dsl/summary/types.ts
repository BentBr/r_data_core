import type { DslStep, FromDef, ToDef, Transform } from '../contracts'

export type { DslStep, FromDef, ToDef, Transform }

export type TranslateFn = (
    key: string,
    params?: Record<string, string> | string,
    fallback?: string
) => string

export type SummaryHandler<T> = (value: T, t: TranslateFn) => string

export type FormatFrom = Extract<FromDef, { type: 'format' }>
export type NonFormatFrom = Exclude<FromDef, { type: 'format' }>
export type FormatTo = Extract<ToDef, { type: 'format' }>
export type NonFormatTo = Exclude<ToDef, { type: 'format' }>
export type FormatToPush = FormatTo & { output: Extract<FormatTo['output'], { mode: 'push' }> }
export type FromSourceType = 'api' | 'uri'

export type TypeMapRecord = Record<string, { type: string }>

export type NonFormatFromSummaryMap = {
    entity: Extract<NonFormatFrom, { type: 'entity' }>
    previous_step: Extract<NonFormatFrom, { type: 'previous_step' }>
    trigger: Extract<NonFormatFrom, { type: 'trigger' }>
}

export type TransformSummaryMap = {
    none: Extract<Transform, { type: 'none' }>
    arithmetic: Extract<Transform, { type: 'arithmetic' }>
    concat: Extract<Transform, { type: 'concat' }>
    build_path: Extract<Transform, { type: 'build_path' }>
    resolve_entity_path: Extract<Transform, { type: 'resolve_entity_path' }>
    get_or_create_entity: Extract<Transform, { type: 'get_or_create_entity' }>
    authenticate: Extract<Transform, { type: 'authenticate' }>
}

export type NonFormatToSummaryMap = {
    entity: Extract<NonFormatTo, { type: 'entity' }>
    next_step: Extract<NonFormatTo, { type: 'next_step' }>
}

export type StepStats = Array<{ label: string; value: string }>
