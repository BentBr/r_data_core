<template>
    <div>
        <div class="text-caption mb-1">{{ t('workflows.dsl.to') }}</div>
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
                    :model-value="(modelValue as any).format?.format_type || 'json'"
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
                        :model-value="
                            (modelValue as any).output?.destination?.destination_type || 'uri'
                        "
                        :items="destinationTypes"
                        :label="t('workflows.dsl.destination_type')"
                        density="comfortable"
                        @update:model-value="updateDestinationType($event)"
                    />
                    <v-select
                        v-if="(modelValue as any).output?.destination?.destination_type === 'uri'"
                        :model-value="(modelValue as any).output?.method || 'POST'"
                        :items="httpMethods"
                        :label="t('workflows.dsl.http_method')"
                        density="comfortable"
                        @update:model-value="updateHttpMethod($event)"
                    />
                </div>
                <v-text-field
                    v-if="(modelValue as any).output?.destination?.destination_type === 'uri'"
                    :model-value="(modelValue as any).output?.destination?.config?.uri || ''"
                    :label="t('workflows.dsl.uri')"
                    density="comfortable"
                    class="mb-2"
                    @update:model-value="updateDestinationConfig('uri', $event)"
                />
                <div class="mb-2">
                    <v-expansion-panels variant="accordion">
                        <v-expansion-panel>
                            <v-expansion-panel-title>{{
                                t('workflows.dsl.auth_type')
                            }}</v-expansion-panel-title>
                            <v-expansion-panel-text>
                                <AuthConfigEditor
                                    :model-value="
                                        (modelValue as any).output?.destination?.auth || {
                                            type: 'none',
                                        }
                                    "
                                    @update:model-value="updateDestinationAuth($event)"
                                />
                            </v-expansion-panel-text>
                        </v-expansion-panel>
                    </v-expansion-panels>
                </div>
            </template>
            <!-- to.api output mode = Provide data via our API endpoint -->
            <template v-if="getOutputMode() === 'api'">
                <div class="text-caption mb-2 pa-2" style="background-color: rgba(var(--v-theme-primary), 0.1); border-radius: 4px;">
                    <strong>{{ t('workflows.dsl.endpoint_info') }}:</strong> GET {{ getFullEndpointUri() }}
                </div>
            </template>
            <div
                v-if="(modelValue as any).format?.format_type === 'csv'"
                class="mb-2"
            >
                <CsvOptionsEditor
                    :model-value="(modelValue as any).format?.options || {}"
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
                >{{ t('workflows.dsl.add_mapping') }}</v-btn
            >
        </template>
        <template v-else-if="modelValue.type === 'entity'">
            <v-select
                :model-value="(modelValue as any).entity_definition"
                :items="entityDefItems"
                item-title="title"
                item-value="value"
                :label="t('workflows.dsl.entity_definition')"
                density="comfortable"
                @update:model-value="onEntityDefChange"
            />
            <v-text-field
                :model-value="(modelValue as any).path"
                :label="t('workflows.dsl.path')"
                density="comfortable"
                @update:model-value="updateField('path', $event)"
            />
            <v-select
                :model-value="(modelValue as any).mode"
                :items="entityModes"
                :label="t('workflows.dsl.mode')"
                density="comfortable"
                @update:model-value="updateField('mode', $event)"
            />
            <v-text-field
                v-if="(modelValue as any).mode === 'update' || (modelValue as any).mode === 'create_or_update'"
                :model-value="(modelValue as any).update_key"
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
                >{{ t('workflows.dsl.add_mapping') }}</v-btn
            >
        </template>
    </div>
</template>

<script setup lang="ts">
    import { ref, watch, onMounted } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useEntityDefinitions } from '@/composables/useEntityDefinitions'
    import { typedHttpClient } from '@/api/typed-client'
    import { env } from '@/env-check'
    import type { ToDef, AuthConfig, HttpMethod } from './dsl-utils'
    import { defaultCsvOptions } from './dsl-utils'
    import CsvOptionsEditor from './CsvOptionsEditor.vue'
    import MappingEditor from './MappingEditor.vue'
    import AuthConfigEditor from './AuthConfigEditor.vue'

    const props = defineProps<{
        modelValue: ToDef
        workflowUuid?: string | null
    }>()

    const emit = defineEmits<{ (e: 'update:modelValue', value: ToDef): void }>()

    const { t } = useTranslations()
    const { entityDefinitions, loadEntityDefinitions } = useEntityDefinitions()

    function getFullEndpointUri(): string {
        const baseUrl = env.apiBaseUrl || window.location.origin
        const uuid = props.workflowUuid || '{workflow-uuid}'
        return `${baseUrl}/api/v1/workflows/${uuid}`
    }

    const entityDefItems = ref<{ title: string; value: string }[]>([])
    const entityTargetFields = ref<string[]>([])
    const mappingEditorRef = ref<{ addEmptyPair: () => void } | null>(null)

    const toTypes = [
        { title: 'Format (CSV/JSON)', value: 'format' },
        { title: 'Entity', value: 'entity' },
    ]
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
            entityDefItems.value = (defs || []).map(d => ({
                title: d.display_name || d.entity_type,
                value: d.entity_type,
            }))
        },
        { immediate: true }
    )

    // Load entity definitions when component is created
    onMounted(() => {
        loadEntityDefinitions()
    })

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
        } catch (e) {
            entityTargetFields.value = []
        }
    }

    // Load entity fields when entity definition is set initially
    watch(
        () => (props.modelValue as any).entity_definition,
        entityType => {
            if (props.modelValue.type === 'entity' && entityType) {
                loadEntityFields(entityType)
            }
        },
        { immediate: true }
    )

    function updateField(field: string, value: any) {
        const updated: any = { ...props.modelValue }
        updated[field] = value
        // Remove 'output' field if type is entity
        if (updated.type === 'entity' && 'output' in updated) {
            delete updated.output
        }
        emit('update:modelValue', updated as ToDef)
    }

    function getOutputMode(): string {
        const output = (props.modelValue as any).output
        if (!output) {
            return 'api'
        }
        if (typeof output === 'string') {
            return output
        }
        if (output.mode) {
            return output.mode
        }
        return 'api'
    }

    function updateFormatType(newType: string) {
        const updated: any = { ...props.modelValue }
        if (!updated.format) {
            updated.format = { format_type: newType, options: {} }
        } else {
            updated.format.format_type = newType
            if (newType === 'csv' && !updated.format.options) {
                updated.format.options = defaultCsvOptions()
            }
        }
        emit('update:modelValue', updated as ToDef)
    }

    function updateFormatOptions(options: any) {
        const updated: any = { ...props.modelValue }
        if (!updated.format) {
            updated.format = { format_type: 'csv', options: {} }
        }
        updated.format.options = options
        emit('update:modelValue', updated as ToDef)
    }

    function updateOutputMode(mode: string) {
        const updated: any = { ...props.modelValue }
        if (mode === 'push') {
            updated.output = {
                mode: 'push',
                destination: {
                    destination_type: 'uri',
                    config: { uri: '' },
                    auth: { type: 'none' },
                },
                method: 'POST',
            }
        } else {
            updated.output = { mode: mode as 'api' | 'download' }
        }
        emit('update:modelValue', updated as ToDef)
    }

    function updateDestinationType(newType: string) {
        const updated: any = { ...props.modelValue }
        if (!updated.output || updated.output.mode !== 'push') {
            updated.output = {
                mode: 'push',
                destination: { destination_type: newType, config: {}, auth: { type: 'none' } },
                method: 'POST',
            }
        } else {
            updated.output.destination.destination_type = newType
            if (newType === 'uri') {
                updated.output.destination.config = { uri: '' }
            } else {
                updated.output.destination.config = {}
            }
        }
        emit('update:modelValue', updated as ToDef)
    }

    function updateDestinationConfig(key: string, value: any) {
        const updated: any = { ...props.modelValue }
        if (!updated.output || updated.output.mode !== 'push') {
            updated.output = {
                mode: 'push',
                destination: { destination_type: 'uri', config: {}, auth: { type: 'none' } },
                method: 'POST',
            }
        }
        if (!updated.output.destination.config) {
            updated.output.destination.config = {}
        }
        updated.output.destination.config[key] = value
        emit('update:modelValue', updated as ToDef)
    }

    function updateHttpMethod(method: HttpMethod) {
        const updated: any = { ...props.modelValue }
        if (!updated.output || updated.output.mode !== 'push') {
            updated.output = {
                mode: 'push',
                destination: {
                    destination_type: 'uri',
                    config: { uri: '' },
                    auth: { type: 'none' },
                },
                method: 'POST',
            }
        }
        updated.output.method = method
        emit('update:modelValue', updated as ToDef)
    }

    function updateDestinationAuth(auth: AuthConfig) {
        const updated: any = { ...props.modelValue }
        if (!updated.output || updated.output.mode !== 'push') {
            updated.output = {
                mode: 'push',
                destination: {
                    destination_type: 'uri',
                    config: { uri: '' },
                    auth: { type: 'none' },
                },
                method: 'POST',
            }
        }
        updated.output.destination.auth = auth
        emit('update:modelValue', updated as ToDef)
    }

    function onTypeChange(newType: 'format' | 'entity') {
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
        } else {
            // Entity type - NO output field
            newTo = {
                type: 'entity',
                entity_definition: '',
                path: '',
                mode: 'create',
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
            const updated: any = { ...props.modelValue }
            if (!updated.mapping) {
                updated.mapping = {}
            }
            updated.mapping[''] = ''
            emit('update:modelValue', updated as ToDef)
        }
    }
</script>

<style scoped>
    .ga-2 {
        gap: 8px;
    }
</style>
