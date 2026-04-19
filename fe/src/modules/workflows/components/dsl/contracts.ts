import type {
    DslStep,
    FromDef,
    ToDef,
    Transform,
    OutputMode,
    HttpMethod,
    Operand,
    StringOperand,
    AuthConfig,
    SourceConfig,
    FormatConfig,
    DestinationConfig,
} from '@/types/schemas/dsl'

export type {
    DslStep,
    FromDef,
    ToDef,
    Transform,
    OutputMode,
    HttpMethod,
    Operand,
    StringOperand,
    AuthConfig,
    SourceConfig,
    FormatConfig,
    DestinationConfig,
}

export type WorkflowTemplate = {
    id: 'api_ingest' | 'remote_pull' | 'entity_export'
    steps: DslStep[]
}
