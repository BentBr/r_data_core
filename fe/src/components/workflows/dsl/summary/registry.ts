import type {
    FormatFrom,
    FormatTo,
    FormatToPush,
    FromSourceType,
    NonFormatFrom,
    NonFormatFromSummaryMap,
    NonFormatTo,
    NonFormatToSummaryMap,
    SummaryHandler,
    Transform,
    TransformSummaryMap,
    TranslateFn,
} from './types'
import {
    createKeyResolver,
    createPartialKeyResolver,
    createTypeResolver,
} from './resolvers'

const fromFormatSourceHandlers: Partial<Record<FromSourceType, SummaryHandler<FormatFrom>>> = {
    api: (from: FormatFrom, t: TranslateFn) =>
        t('workflows.dsl.summary.from.api', {
            format: from.format.format_type.toUpperCase(),
        }),
    uri: (from: FormatFrom, t: TranslateFn) => {
        const uri =
            typeof from.source.config.uri === 'string' && from.source.config.uri.trim()
                ? from.source.config.uri
                : t('workflows.dsl.summary.placeholders.remote_url')
        return t('workflows.dsl.summary.from.uri', {
            format: from.format.format_type.toUpperCase(),
            uri,
        })
    },
}

const fromSummaryHandlers: {
    [K in keyof NonFormatFromSummaryMap]: SummaryHandler<NonFormatFromSummaryMap[K]>
} = {
    entity: (from: Extract<NonFormatFrom, { type: 'entity' }>, t: TranslateFn) =>
        from.entity_definition.trim()
            ? t('workflows.dsl.summary.from.entity_named', {
                  entityDefinition: from.entity_definition,
              })
            : t('workflows.dsl.summary.from.entity'),
    previous_step: (
        _from: Extract<NonFormatFrom, { type: 'previous_step' }>,
        t: TranslateFn
    ) => t('workflows.dsl.summary.from.previous_step'),
    trigger: (_from: Extract<NonFormatFrom, { type: 'trigger' }>, t: TranslateFn) =>
        t('workflows.dsl.summary.from.trigger'),
}

const transformSummaryHandlers: {
    [K in keyof TransformSummaryMap]: SummaryHandler<TransformSummaryMap[K]>
} = {
    none: (_transform: Extract<Transform, { type: 'none' }>, t: TranslateFn) =>
        t('workflows.dsl.summary.transform.none'),
    arithmetic: (transform: Extract<Transform, { type: 'arithmetic' }>, t: TranslateFn) =>
        t('workflows.dsl.summary.transform.arithmetic', {
            target: transform.target || t('workflows.dsl.summary.placeholders.numeric_field'),
        }),
    concat: (transform: Extract<Transform, { type: 'concat' }>, t: TranslateFn) =>
        t('workflows.dsl.summary.transform.concat', {
            target: transform.target || t('workflows.dsl.summary.placeholders.text_field'),
        }),
    build_path: (transform: Extract<Transform, { type: 'build_path' }>, t: TranslateFn) =>
        t('workflows.dsl.summary.transform.build_path', {
            target: transform.target || t('workflows.dsl.summary.placeholders.target_field'),
        }),
    resolve_entity_path: (
        transform: Extract<Transform, { type: 'resolve_entity_path' }>,
        t: TranslateFn
    ) =>
        t('workflows.dsl.summary.transform.resolve_entity_path', {
            entityType: transform.entity_type,
        }),
    get_or_create_entity: (
        transform: Extract<Transform, { type: 'get_or_create_entity' }>,
        t: TranslateFn
    ) =>
        t('workflows.dsl.summary.transform.get_or_create_entity', {
            entityType: transform.entity_type,
        }),
}

const toFormatOutputHandlers = {
    api: (to: FormatTo, t: TranslateFn) =>
        t('workflows.dsl.summary.to.api', {
            format: to.format.format_type.toUpperCase(),
        }),
    download: (to: FormatTo, t: TranslateFn) =>
        t('workflows.dsl.summary.to.download', {
            format: to.format.format_type.toUpperCase(),
        }),
    push: (to: FormatToPush, t: TranslateFn) => {
        const uri =
            typeof to.output.destination.config.uri === 'string' &&
            to.output.destination.config.uri.trim()
                ? to.output.destination.config.uri
                : t('workflows.dsl.summary.placeholders.remote_url')
        return t('workflows.dsl.summary.to.push', {
            format: to.format.format_type.toUpperCase(),
            uri,
        })
    },
} satisfies {
    api: SummaryHandler<FormatTo>
    download: SummaryHandler<FormatTo>
    push: SummaryHandler<FormatToPush>
}

const toSummaryHandlers: {
    [K in keyof NonFormatToSummaryMap]: SummaryHandler<NonFormatToSummaryMap[K]>
} = {
    entity: (to: Extract<NonFormatTo, { type: 'entity' }>, t: TranslateFn) =>
        to.entity_definition.trim()
            ? t('workflows.dsl.summary.to.entity_named', {
                  entityDefinition: to.entity_definition,
              })
            : t('workflows.dsl.summary.to.entity'),
    next_step: (_to: Extract<NonFormatTo, { type: 'next_step' }>, t: TranslateFn) =>
        t('workflows.dsl.summary.to.next_step'),
}

export const resolveFromSummary = createTypeResolver<NonFormatFromSummaryMap>(fromSummaryHandlers)
export const resolveFromFormatSource = createPartialKeyResolver<FromSourceType, FormatFrom>(
    fromFormatSourceHandlers
)
export const resolveTransformSummary =
    createTypeResolver<TransformSummaryMap>(transformSummaryHandlers)
export const resolveToSummary = createTypeResolver<NonFormatToSummaryMap>(toSummaryHandlers)
export const resolveToFormatOutput = createKeyResolver<'api' | 'download' | 'push', FormatTo | FormatToPush>({
    api: (to, t) => toFormatOutputHandlers.api(to as FormatTo, t),
    download: (to, t) => toFormatOutputHandlers.download(to as FormatTo, t),
    push: (to, t) => toFormatOutputHandlers.push(to as FormatToPush, t),
})
