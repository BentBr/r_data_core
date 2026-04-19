import type { SummaryHandler, TranslateFn, TypeMapRecord } from './types'

export function createTypeResolver<T extends TypeMapRecord>(handlers: {
    [K in keyof T]: SummaryHandler<T[K]>
}) {
    return <K extends keyof T>(value: T[K], t: TranslateFn): string => {
        const handler = handlers[value.type as K] as SummaryHandler<T[K]> | undefined
        if (!handler) {
            return value.type
        }
        return handler(value, t)
    }
}

export function createKeyResolver<K extends string, T>(handlers: Record<K, SummaryHandler<T>>) {
    return (key: K, value: T, t: TranslateFn): string => handlers[key](value, t)
}

export function createPartialKeyResolver<K extends string, T>(
    handlers: Partial<Record<K, SummaryHandler<T>>>
) {
    return (key: K, value: T, t: TranslateFn): string | undefined => handlers[key]?.(value, t)
}
