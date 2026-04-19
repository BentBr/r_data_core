import type {
    DslStep,
    FormatToPush,
    FromDef,
    StepStats,
    ToDef,
    Transform,
    TranslateFn,
} from './types'
import {
    resolveFromFormatSource,
    resolveFromSummary,
    resolveToFormatOutput,
    resolveToSummary,
    resolveTransformSummary,
} from './registry'

function mappingCount(mapping: Record<string, string>): number {
    return Object.entries(mapping).filter(([k, v]) => k.trim() || v.trim()).length
}

export function describeFrom(from: FromDef, t: TranslateFn): string {
    if (from.type === 'format') {
        const sourceType = from.source.source_type as 'api' | 'uri'
        const resolved = resolveFromFormatSource(sourceType, from, t)
        if (resolved) {
            return resolved
        }
        return t('workflows.dsl.summary.from.generic', {
            format: from.format.format_type.toUpperCase(),
            sourceType: from.source.source_type,
        })
    }

    return resolveFromSummary(from, t)
}

export function describeTransform(transform: Transform, t: TranslateFn): string {
    return resolveTransformSummary(transform, t)
}

export function describeTo(to: ToDef, t: TranslateFn): string {
    if (to.type === 'format') {
        return resolveToFormatOutput(
            to.output.mode,
            to.output.mode === 'push' ? (to as FormatToPush) : to,
            t
        )
    }

    return resolveToSummary(to, t)
}

export function buildStepSummary(step: DslStep, t: TranslateFn): string {
    return t('workflows.dsl.summary.step', {
        from: describeFrom(step.from, t),
        transform: describeTransform(step.transform, t),
        to: describeTo(step.to, t),
    })
}

export function getStepStats(step: DslStep, t: TranslateFn): StepStats {
    const stats: StepStats = [
        {
            label: t('workflows.dsl.stats.input_mappings'),
            value: String(mappingCount(step.from.mapping)),
        },
        {
            label: t('workflows.dsl.stats.output_mappings'),
            value: String(mappingCount(step.to.mapping)),
        },
    ]

    if (step.transform.type !== 'none') {
        stats.push({
            label: t('workflows.dsl.stats.transform'),
            value: t(
                `workflows.dsl.transform_names.${step.transform.type}`,
                step.transform.type.replace(/_/g, ' ')
            ),
        })
    }

    return stats
}

export type { TranslateFn, StepStats } from './types'
