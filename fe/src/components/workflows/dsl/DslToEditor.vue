<template>
    <div>
        <div class="text-subtitle-2 font-weight-bold mb-2">{{ t('workflows.dsl.to') }}</div>
        <v-select
            :model-value="modelValue.type"
            :items="toTypes"
            :label="t('workflows.dsl.to_type')"
            density="comfortable"
            class="mb-2"
            @update:model-value="onTypeChange"
        />
        <template v-if="modelValue.type === 'format'">
            <div class="d-flex ga-2 mb-2 flex-wrap">
                <v-select
                    :model-value="formatType"
                    :items="formatTypes"
                    :label="t('workflows.dsl.format_type')"
                    density="comfortable"
                    @update:model-value="updateFormatType($event)"
                />
                <v-select
                    :model-value="getOutputMode()"
                    :items="outputModes"
                    :label="t('workflows.dsl.output')"
                    density="comfortable"
                    @update:model-value="updateOutputMode($event)"
                />
            </div>
            <template v-if="getOutputMode() === 'push'">
                <div class="d-flex ga-2 mb-2 flex-wrap">
                    <v-select
                        :model-value="outputDestinationType"
                        :items="destinationTypes"
                        :label="t('workflows.dsl.destination_type')"
                        density="comfortable"
                        @update:model-value="updateDestinationType($event)"
                    />
                    <v-select
                        v-if="outputDestinationType === 'uri'"
                        :model-value="outputMethod"
                        :items="httpMethods"
                        :label="t('workflows.dsl.http_method')"
                        density="comfortable"
                        @update:model-value="updateHttpMethod($event)"
                    />
                </div>
                <v-text-field
                    v-if="outputDestinationType === 'uri'"
                    :model-value="outputDestinationUri"
                    :label="t('workflows.dsl.uri')"
                    density="comfortable"
                    class="mb-2"
                    @update:model-value="updateDestinationConfig('uri', $event)"
                />
                <div class="mb-2">
                    <v-expansion-panels variant="accordion">
                        <v-expansion-panel>
                            <v-expansion-panel-title
                                >{{ t('workflows.dsl.auth_type') }}
                            </v-expansion-panel-title>
                            <v-expansion-panel-text>
                                <AuthConfigEditor
                                    :model-value="outputDestinationAuth"
                                    @update:model-value="updateDestinationAuth($event)"
                                />
                            </v-expansion-panel-text>
                        </v-expansion-panel>
                    </v-expansion-panels>
                </div>
            </template>
            <!-- to.api output mode = Provide data via our API endpoint -->
            <template v-if="getOutputMode() === 'api'">
                <div
                    class="text-caption mb-2 pa-2"
                    style="background-color: rgba(var(--v-theme-primary), 0.1); border-radius: 4px"
                >
                    <strong>{{ t('workflows.dsl.endpoint_info') }}:</strong> GET
                    {{ getFullEndpointUri() }}
                </div>
            </template>
            <div
                v-if="formatType === 'csv'"
                class="mb-2"
            >
                <CsvOptionsEditor
                    :model-value="formatOptions"
                    @update:model-value="updateFormatOptions($event)"
                />
            </div>
            <div class="text-caption mb-1 mt-2">
                {{ t('workflows.dsl.mapping_normalized_destination') }}
            </div>
            <MappingEditor
                :model-value="modelValue.mapping"
                :left-label="t('workflows.dsl.normalized')"
                :right-label="t('workflows.dsl.destination')"
                @update:model-value="updateField('mapping', $event)"
            />
            <v-btn
                size="x-small"
                variant="tonal"
                @click="addMapping"
                >{{ t('workflows.dsl.add_mapping') }}
            </v-btn>
        </template>
        <template v-else-if="modelValue.type === 'entity'">
            <v-select
                :model-value="entityDefinition"
                :items="entityDefItems"
                item-title="title"
                item-value="value"
                :label="t('workflows.dsl.entity_definition')"
                density="comfortable"
                @update:model-value="onEntityDefChange"
            />
            <v-text-field
                :model-value="entityPath"
                :label="t('workflows.dsl.path')"
                density="comfortable"
                @update:model-value="updateField('path', $event)"
            />
            <v-select
                :model-value="entityMode"
                :items="entityModes"
                :label="t('workflows.dsl.mode')"
                density="comfortable"
                @update:model-value="updateField('mode', $event)"
            />
            <v-text-field
                v-if="entityMode === 'update' || entityMode === 'create_or_update'"
                :model-value="entityUpdateKey"
                :label="t('workflows.dsl.update_key')"
                density="comfortable"
                @update:model-value="updateField('update_key', $event)"
            />
            <div class="text-caption mb-1 mt-2">
                {{ t('workflows.dsl.mapping_normalized_destination') }}
            </div>
            <MappingEditor
                ref="mappingEditorRef"
                :model-value="modelValue.mapping"
                :left-label="t('workflows.dsl.entity_field')"
                :right-label="t('workflows.dsl.normalized_field')"
                :left-items="entityTargetFields"
                :use-select-for-left="true"
                @update:model-value="updateField('mapping', $event)"
            />
            <v-btn
                size="x-small"
                variant="tonal"
                @click="addMapping"
                >{{ t('workflows.dsl.add_mapping') }}
            </v-btn>
        </template>
        <template v-else-if="modelValue.type === 'next_step'">
            <!-- Info banner -->
            <v-alert
                v-if="isLastStep"
                type="error"
                density="compact"
                class="mb-2"
            >
                {{ t('workflows.dsl.next_step_error_last_step') }}
            </v-alert>
            <div
                v-else
                class="text-caption mb-2 pa-2"
                style="background-color: rgba(var(--v-theme-info), 0.1); border-radius: 4px"
            >
                <v-icon
                    size="small"
                    class="mr-1"
                    >mdi-arrow-down-circle
                </v-icon>
                {{ t('workflows.dsl.next_step_info') }}
            </div>
            <div class="text-caption mb-1 mt-2">
                {{ t('workflows.dsl.mapping_normalized_next_step') }}
            </div>
            <MappingEditor
                ref="mappingEditorRef"
                :model-value="modelValue.mapping"
                :left-label="t('workflows.dsl.normalized')"
                :right-label="t('workflows.dsl.next_step_field')"
                @update:model-value="updateField('mapping', $event)"
            />
            <v-btn
                size="x-small"
                variant="tonal"
                @click="addMapping"
                >{{ t('workflows.dsl.add_mapping') }}
            </v-btn>
        </template>
        <template v-else-if="modelValue.type === 'email'">
            <v-alert
                type="info"
                variant="tonal"
                density="compact"
                class="mb-3"
            >
                {{ t('workflows.dsl.hints.email_to.info') }}
            </v-alert>

            <v-select
                :model-value="emailTemplateUuid"
                :items="emailTemplateItems"
                item-title="title"
                item-value="value"
                :label="t('workflows.dsl.send_email_template')"
                density="comfortable"
                class="mb-2"
                :hint="t('workflows.dsl.hints.email_to.template_uuid')"
                persistent-hint
                @update:model-value="updateEmailField('template_uuid', $event)"
            />

            <div class="text-caption mb-1 mt-2">{{ t('workflows.dsl.send_email_to') }}</div>
            <div
                v-for="(operand, idx) in emailToOperands"
                :key="`email-to-${idx}`"
                class="d-flex ga-2 mb-2 align-center"
            >
                <v-select
                    :model-value="operand.kind"
                    :items="stringOperandKinds"
                    :label="t('workflows.dsl.value_kind')"
                    density="comfortable"
                    class="flex-0"
                    style="max-width: 180px"
                    @update:model-value="updateEmailToKind(idx, $event)"
                />
                <v-text-field
                    v-if="operand.kind === 'field'"
                    :model-value="operand.field"
                    :label="t('workflows.dsl.value')"
                    density="comfortable"
                    @update:model-value="updateEmailToField(idx, $event)"
                />
                <v-text-field
                    v-else
                    :model-value="operand.value"
                    :label="t('workflows.dsl.value')"
                    density="comfortable"
                    @update:model-value="updateEmailToValue(idx, $event)"
                />
                <v-btn
                    icon="mdi-delete"
                    variant="text"
                    size="small"
                    @click="removeEmailTo(idx)"
                />
            </div>
            <v-btn
                variant="outlined"
                size="small"
                class="mb-2"
                @click="addEmailTo"
            >
                {{ t('workflows.dsl.add_recipient') }}
            </v-btn>
            <div class="text-caption text-medium-emphasis mb-4">
                {{ t('workflows.dsl.hints.email_to.to') }}
            </div>

            <div class="text-caption mb-1 mt-2">{{ t('workflows.dsl.send_email_cc') }}</div>
            <div
                v-for="(operand, idx) in emailCcOperands"
                :key="`email-cc-${idx}`"
                class="d-flex ga-2 mb-2 align-center"
            >
                <v-select
                    :model-value="operand.kind"
                    :items="stringOperandKinds"
                    :label="t('workflows.dsl.value_kind')"
                    density="comfortable"
                    class="flex-0"
                    style="max-width: 180px"
                    @update:model-value="updateEmailCcKind(idx, $event)"
                />
                <v-text-field
                    v-if="operand.kind === 'field'"
                    :model-value="operand.field"
                    :label="t('workflows.dsl.value')"
                    density="comfortable"
                    @update:model-value="updateEmailCcField(idx, $event)"
                />
                <v-text-field
                    v-else
                    :model-value="operand.value"
                    :label="t('workflows.dsl.value')"
                    density="comfortable"
                    @update:model-value="updateEmailCcValue(idx, $event)"
                />
                <v-btn
                    icon="mdi-delete"
                    variant="text"
                    size="small"
                    @click="removeEmailCc(idx)"
                />
            </div>
            <v-btn
                variant="outlined"
                size="small"
                class="mb-2"
                @click="addEmailCc"
            >
                {{ t('workflows.dsl.add_cc_recipient') }}
            </v-btn>
            <div class="text-caption text-medium-emphasis mb-4">
                {{ t('workflows.dsl.hints.email_to.cc') }}
            </div>

            <div class="text-caption mb-1 mt-2">
                {{ t('workflows.dsl.mapping_normalized_destination') }}
            </div>
            <div class="text-caption text-medium-emphasis mb-2">
                {{ t('workflows.dsl.hints.email_to.mapping') }}
            </div>
            <MappingEditor
                ref="mappingEditorRef"
                :model-value="emailMapping"
                :left-label="t('workflows.dsl.normalized')"
                :right-label="t('workflows.dsl.destination')"
                @update:model-value="updateEmailField('mapping', $event)"
            />
            <v-btn
                size="x-small"
                variant="tonal"
                @click="addMapping"
                >{{ t('workflows.dsl.add_mapping') }}
            </v-btn>
        </template>
    </div>
</template>

<script setup lang="ts">
    import { ref, watch, onMounted, computed } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useEntityDefinitions } from '@/composables/useEntityDefinitions'
    import { useCapabilitiesStore } from '@/stores/capabilities'
    import { typedHttpClient } from '@/api/typed-client'
    import { buildApiUrl } from '@/env-check'
    import type { ToDef, AuthConfig, HttpMethod, OutputMode, StringOperand } from './contracts'
    import { defaultCsvOptions } from './dsl-utils'
    import CsvOptionsEditor from './CsvOptionsEditor.vue'
    import MappingEditor from './MappingEditor.vue'
    import AuthConfigEditor from './AuthConfigEditor.vue'

    const props = defineProps<{
        modelValue: ToDef
        workflowUuid?: string | null
        isLastStep?: boolean
    }>()

    const emit = defineEmits<{ (e: 'update:modelValue', value: ToDef): void }>()

    const { t } = useTranslations()
    const capabilitiesStore = useCapabilitiesStore()
    const { entityDefinitions, loadEntityDefinitions } = useEntityDefinitions()

    function getFullEndpointUri(): string {
        const uuid = props.workflowUuid ?? '{workflow-uuid}'
        return buildApiUrl(`/api/v1/workflows/${uuid}`)
    }

    const entityDefItems = ref<{ title: string; value: string }[]>([])
    const entityTargetFields = ref<string[]>([])
    const mappingEditorRef = ref<{ addEmptyPair: () => void } | null>(null)

    // Computed properties to avoid 'as any' in templates
    const formatType = computed(() => {
        if (props.modelValue.type === 'format') {
            return props.modelValue.format.format_type
        }
        return 'json'
    })

    const formatOptions = computed(() => {
        if (props.modelValue.type === 'format') {
            return props.modelValue.format.options ?? {}
        }
        return {}
    })

    const outputDestinationType = computed(() => {
        if (props.modelValue.type === 'format') {
            const output = props.modelValue.output
            if (
                typeof output === 'object' &&
                'mode' in output &&
                output.mode === 'push' &&
                'destination' in output
            ) {
                return output.destination.destination_type
            }
        }
        return 'uri'
    })

    const outputMethod = computed(() => {
        if (props.modelValue.type === 'format') {
            const output = props.modelValue.output
            if (
                typeof output === 'object' &&
                'mode' in output &&
                output.mode === 'push' &&
                'method' in output
            ) {
                return output.method ?? 'POST'
            }
        }
        return 'POST'
    })

    const outputDestinationUri = computed(() => {
        if (props.modelValue.type === 'format') {
            const output = props.modelValue.output
            if (
                typeof output === 'object' &&
                'mode' in output &&
                output.mode === 'push' &&
                'destination' in output &&
                'config' in output.destination &&
                typeof output.destination.config === 'object' &&
                'uri' in output.destination.config
            ) {
                return String(output.destination.config.uri)
            }
        }
        return ''
    })

    const outputDestinationAuth = computed((): AuthConfig => {
        if (props.modelValue.type === 'format') {
            const output = props.modelValue.output
            if (
                typeof output === 'object' &&
                'mode' in output &&
                output.mode === 'push' &&
                'destination' in output &&
                'auth' in output.destination
            ) {
                return output.destination.auth ?? { type: 'none' }
            }
        }
        return { type: 'none' }
    })

    const entityDefinition = computed(() => {
        if (props.modelValue.type === 'entity') {
            return props.modelValue.entity_definition
        }
        return ''
    })

    const entityPath = computed(() => {
        if (props.modelValue.type === 'entity') {
            return props.modelValue.path
        }
        return ''
    })

    const entityMode = computed(() => {
        if (props.modelValue.type === 'entity') {
            return props.modelValue.mode
        }
        return 'create'
    })

    const entityUpdateKey = computed(() => {
        if (props.modelValue.type === 'entity') {
            return props.modelValue.update_key ?? ''
        }
        return ''
    })

    const stringOperandKinds = ['field', 'const_string']

    const toTypes = computed(() => {
        const types = [
            { title: 'Format (CSV/JSON)', value: 'format' },
            { title: 'Entity', value: 'entity' },
            { title: 'Next Step', value: 'next_step' },
        ]
        if (capabilitiesStore.workflowMailConfigured) {
            types.push({ title: 'Email', value: 'email' })
        }
        return types
    })

    const emailTemplateItems = ref<{ title: string; value: string }[]>([])

    const emailTemplateUuid = computed(() => {
        if (props.modelValue.type === 'email') {
            return props.modelValue.template_uuid
        }
        return ''
    })

    const emailToOperands = computed((): StringOperand[] => {
        if (props.modelValue.type === 'email') {
            return props.modelValue.to
        }
        return []
    })

    const emailCcOperands = computed((): StringOperand[] => {
        if (props.modelValue.type === 'email') {
            return props.modelValue.cc ?? []
        }
        return []
    })

    const emailMapping = computed(() => {
        if (props.modelValue.type === 'email') {
            return props.modelValue.mapping
        }
        return {}
    })

    // Computed property for isLastStep
    const isLastStep = computed(() => props.isLastStep)
    const outputModes = [
        { title: 'API', value: 'api' },
        { title: 'Download', value: 'download' },
        { title: 'Push', value: 'push' },
    ]
    const formatTypes = [
        { title: 'CSV', value: 'csv' },
        { title: 'JSON', value: 'json' },
    ]
    const destinationTypes = [{ title: 'URI', value: 'uri' }]
    const httpMethods: { title: string; value: HttpMethod }[] = [
        { title: 'GET', value: 'GET' },
        { title: 'POST', value: 'POST' },
        { title: 'PUT', value: 'PUT' },
        { title: 'PATCH', value: 'PATCH' },
        { title: 'DELETE', value: 'DELETE' },
        { title: 'HEAD', value: 'HEAD' },
        { title: 'OPTIONS', value: 'OPTIONS' },
    ]
    const entityModes = [
        { title: 'Create', value: 'create' },
        { title: 'Update', value: 'update' },
        { title: 'Create or Update', value: 'create_or_update' },
    ]

    // Load entity definitions on mount
    watch(
        () => entityDefinitions.value,
        defs => {
            entityDefItems.value = defs.map(d => ({
                title: d.display_name || d.entity_type,
                value: d.entity_type,
            }))
        },
        { immediate: true }
    )

    // Load entity definitions when component is created
    onMounted(() => {
        void loadEntityDefinitions()
        void loadEmailTemplates()
    })

    async function loadEmailTemplates() {
        try {
            const templates = await typedHttpClient.listEmailTemplates('workflow')
            emailTemplateItems.value = templates.map(tmpl => ({
                title: tmpl.name,
                value: tmpl.uuid,
            }))
        } catch {
            emailTemplateItems.value = []
        }
    }

    // Load entity fields when entity definition changes
    async function onEntityDefChange(entityType: string) {
        updateField('entity_definition', entityType)
        await loadEntityFields(entityType)
    }

    // Load entity fields helper
    async function loadEntityFields(entityType: string | null | undefined) {
        if (!entityType) {
            entityTargetFields.value = []
            return
        }
        try {
            const fields = await typedHttpClient.getEntityFields(entityType)
            // Filter out system fields that shouldn't be manually set
            const systemFields = [
                'uuid',
                'updated_at',
                'updated_by',
                'created_at',
                'created_by',
                'version',
            ]
            entityTargetFields.value = fields
                .map(f => f.name)
                .filter(name => !systemFields.includes(name))
        } catch {
            entityTargetFields.value = []
        }
    }

    // Load entity fields when entity definition is set initially
    watch(
        () => (props.modelValue.type === 'entity' ? props.modelValue.entity_definition : undefined),
        entityType => {
            if (props.modelValue.type === 'entity' && entityType) {
                void loadEntityFields(entityType)
            }
        },
        { immediate: true }
    )

    function updateField(field: string, value: unknown) {
        const updated = { ...props.modelValue } as Record<string, unknown>
        updated[field] = value
        // Remove 'output' field if type is entity
        if (updated.type === 'entity' && 'output' in updated) {
            delete updated.output
        }
        emit('update:modelValue', updated as ToDef)
    }

    function getOutputMode(): string {
        if (props.modelValue.type === 'format') {
            const output = props.modelValue.output
            if (typeof output === 'string') {
                return output
            }
            if ('mode' in output) {
                return output.mode
            }
        }
        return 'api'
    }

    function updateFormatType(newType: string) {
        if (props.modelValue.type !== 'format') {
            return
        }
        const updated: ToDef = {
            ...props.modelValue,
            format: {
                ...props.modelValue.format,
                format_type: newType,
                options:
                    newType === 'csv' && !props.modelValue.format.options
                        ? defaultCsvOptions()
                        : props.modelValue.format.options,
            },
        }
        emit('update:modelValue', updated)
    }

    function updateFormatOptions(options: Record<string, unknown>) {
        if (props.modelValue.type !== 'format') {
            return
        }
        const updated: ToDef = {
            ...props.modelValue,
            format: {
                ...props.modelValue.format,
                options,
            },
        }
        emit('update:modelValue', updated)
    }

    function updateOutputMode(mode: string) {
        if (props.modelValue.type !== 'format') {
            return
        }
        let output: OutputMode
        if (mode === 'push') {
            output = {
                mode: 'push',
                destination: {
                    destination_type: 'uri',
                    config: { uri: '' },
                    auth: { type: 'none' },
                },
                method: 'POST',
            }
        } else if (mode === 'download') {
            output = { mode: 'download' }
        } else {
            output = { mode: 'api' }
        }
        const updated: ToDef = {
            ...props.modelValue,
            output,
        }
        emit('update:modelValue', updated)
    }

    function updateDestinationType(newType: string) {
        if (props.modelValue.type !== 'format') {
            return
        }
        const currentOutput = props.modelValue.output
        let output: OutputMode
        if (
            typeof currentOutput === 'object' &&
            'mode' in currentOutput &&
            currentOutput.mode === 'push'
        ) {
            output = {
                ...currentOutput,
                destination: {
                    ...currentOutput.destination,
                    destination_type: newType,
                    config: newType === 'uri' ? { uri: '' } : {},
                },
            }
        } else {
            output = {
                mode: 'push',
                destination: { destination_type: newType, config: {}, auth: { type: 'none' } },
                method: 'POST',
            }
        }
        const updated: ToDef = {
            ...props.modelValue,
            output,
        }
        emit('update:modelValue', updated)
    }

    function updateDestinationConfig(key: string, value: unknown) {
        if (props.modelValue.type !== 'format') {
            return
        }
        const currentOutput = props.modelValue.output
        let output: OutputMode
        if (
            typeof currentOutput === 'object' &&
            'mode' in currentOutput &&
            currentOutput.mode === 'push'
        ) {
            output = {
                ...currentOutput,
                destination: {
                    ...currentOutput.destination,
                    config: {
                        ...currentOutput.destination.config,
                        [key]: value,
                    },
                },
            }
        } else {
            output = {
                mode: 'push',
                destination: {
                    destination_type: 'uri',
                    config: { [key]: value },
                    auth: { type: 'none' },
                },
                method: 'POST',
            }
        }
        const updated: ToDef = {
            ...props.modelValue,
            output,
        }
        emit('update:modelValue', updated)
    }

    function updateHttpMethod(method: HttpMethod) {
        if (props.modelValue.type !== 'format') {
            return
        }
        const currentOutput = props.modelValue.output
        let output: OutputMode
        if (
            typeof currentOutput === 'object' &&
            'mode' in currentOutput &&
            currentOutput.mode === 'push'
        ) {
            output = {
                ...currentOutput,
                method,
            }
        } else {
            output = {
                mode: 'push',
                destination: {
                    destination_type: 'uri',
                    config: { uri: '' },
                    auth: { type: 'none' },
                },
                method,
            }
        }
        const updated: ToDef = {
            ...props.modelValue,
            output,
        }
        emit('update:modelValue', updated)
    }

    function updateDestinationAuth(auth: AuthConfig) {
        if (props.modelValue.type !== 'format') {
            return
        }
        const currentOutput = props.modelValue.output
        let output: OutputMode
        if (
            typeof currentOutput === 'object' &&
            'mode' in currentOutput &&
            currentOutput.mode === 'push'
        ) {
            output = {
                ...currentOutput,
                destination: {
                    ...currentOutput.destination,
                    auth,
                },
            }
        } else {
            output = {
                mode: 'push',
                destination: {
                    destination_type: 'uri',
                    config: { uri: '' },
                    auth,
                },
                method: 'POST',
            }
        }
        const updated: ToDef = {
            ...props.modelValue,
            output,
        }
        emit('update:modelValue', updated)
    }

    function onTypeChange(newType: 'format' | 'entity' | 'next_step' | 'email') {
        let newTo: ToDef
        if (newType === 'format') {
            newTo = {
                type: 'format',
                output: { mode: 'api' },
                format: {
                    format_type: 'json',
                    options: {},
                },
                mapping: {},
            }
        } else if (newType === 'entity') {
            // Entity type - NO output field, path is optional (derived from parent_uuid if not set)
            newTo = {
                type: 'entity',
                entity_definition: '',
                mode: 'create',
                mapping: {},
            }
        } else if (newType === 'email') {
            newTo = {
                type: 'email',
                template_uuid: '',
                to: [],
                mapping: {},
            }
        } else {
            // next_step
            newTo = {
                type: 'next_step',
                mapping: {},
            }
        }
        emit('update:modelValue', newTo)
    }

    function addMapping() {
        // Add empty pair directly to MappingEditor's local state
        if (mappingEditorRef.value) {
            mappingEditorRef.value.addEmptyPair()
        } else {
            // Fallback: add to mapping object
            const updated: ToDef = {
                ...props.modelValue,
                mapping: {
                    ...props.modelValue.mapping,
                    '': '',
                },
            }
            emit('update:modelValue', updated)
        }
    }

    // Email to-target field helper
    function updateEmailField(field: string, value: unknown) {
        if (props.modelValue.type === 'email') {
            emit('update:modelValue', { ...props.modelValue, [field]: value })
        }
    }

    // Email To operands
    function addEmailTo() {
        if (props.modelValue.type === 'email') {
            const newOperand: StringOperand = { kind: 'const_string', value: '' }
            emit('update:modelValue', {
                ...props.modelValue,
                to: [...props.modelValue.to, newOperand],
            })
        }
    }

    function removeEmailTo(idx: number) {
        if (props.modelValue.type === 'email') {
            emit('update:modelValue', {
                ...props.modelValue,
                to: props.modelValue.to.filter((_, i) => i !== idx),
            })
        }
    }

    function updateEmailToKind(idx: number, kind: 'field' | 'const_string') {
        if (props.modelValue.type === 'email') {
            const updated = props.modelValue.to.map((op, i) => {
                if (i !== idx) return op
                return kind === 'field'
                    ? ({ kind: 'field', field: '' } as StringOperand)
                    : ({ kind: 'const_string', value: '' } as StringOperand)
            })
            emit('update:modelValue', { ...props.modelValue, to: updated })
        }
    }

    function updateEmailToField(idx: number, fieldValue: string) {
        if (props.modelValue.type === 'email') {
            const updated = props.modelValue.to.map((op, i) => {
                if (i !== idx) return op
                return { kind: 'field' as const, field: fieldValue }
            })
            emit('update:modelValue', { ...props.modelValue, to: updated })
        }
    }

    function updateEmailToValue(idx: number, val: string) {
        if (props.modelValue.type === 'email') {
            const updated = props.modelValue.to.map((op, i) => {
                if (i !== idx) return op
                return { kind: 'const_string' as const, value: val }
            })
            emit('update:modelValue', { ...props.modelValue, to: updated })
        }
    }

    // Email CC operands
    function addEmailCc() {
        if (props.modelValue.type === 'email') {
            const newOperand: StringOperand = { kind: 'const_string', value: '' }
            const existing = props.modelValue.cc ?? []
            emit('update:modelValue', {
                ...props.modelValue,
                cc: [...existing, newOperand],
            })
        }
    }

    function removeEmailCc(idx: number) {
        if (props.modelValue.type === 'email') {
            const existing = props.modelValue.cc ?? []
            const updated = existing.filter((_, i) => i !== idx)
            emit('update:modelValue', {
                ...props.modelValue,
                cc: updated.length > 0 ? updated : undefined,
            })
        }
    }

    function updateEmailCcKind(idx: number, kind: 'field' | 'const_string') {
        if (props.modelValue.type === 'email') {
            const existing = props.modelValue.cc ?? []
            const updated = existing.map((op, i) => {
                if (i !== idx) return op
                return kind === 'field'
                    ? ({ kind: 'field', field: '' } as StringOperand)
                    : ({ kind: 'const_string', value: '' } as StringOperand)
            })
            emit('update:modelValue', { ...props.modelValue, cc: updated })
        }
    }

    function updateEmailCcField(idx: number, fieldValue: string) {
        if (props.modelValue.type === 'email') {
            const existing = props.modelValue.cc ?? []
            const updated = existing.map((op, i) => {
                if (i !== idx) return op
                return { kind: 'field' as const, field: fieldValue }
            })
            emit('update:modelValue', { ...props.modelValue, cc: updated })
        }
    }

    function updateEmailCcValue(idx: number, val: string) {
        if (props.modelValue.type === 'email') {
            const existing = props.modelValue.cc ?? []
            const updated = existing.map((op, i) => {
                if (i !== idx) return op
                return { kind: 'const_string' as const, value: val }
            })
            emit('update:modelValue', { ...props.modelValue, cc: updated })
        }
    }
</script>

<style scoped>
    .ga-2 {
        gap: 8px;
    }
</style>
