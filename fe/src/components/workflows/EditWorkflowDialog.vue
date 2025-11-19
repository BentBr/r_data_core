<template>
    <v-dialog
        v-model="model"
        max-width="1200px"
    >
        <v-card>
            <v-card-title>Edit Workflow</v-card-title>
            <v-card-text>
                <v-tabs v-model="activeTab">
                    <v-tab value="edit">Edit</v-tab>
                    <v-tab value="history">{{ t('entities.details.history') }}</v-tab>
                </v-tabs>

                <v-window v-model="activeTab">
                    <v-window-item value="edit">
                        <v-form
                            ref="formRef"
                            class="mt-4"
                            @submit.prevent
                        >
                            <v-text-field
                                v-model="form.name"
                                label="Name"
                                :rules="[rules.required]"
                            />
                            <v-textarea
                                v-model="form.description"
                                label="Description"
                                rows="2"
                                auto-grow
                            />
                            <v-select
                                v-model="form.kind"
                                label="Kind"
                                :items="kinds"
                                item-title="label"
                                item-value="value"
                            />
                            <v-switch
                                v-model="form.enabled"
                                :label="t('workflows.create.enabled')"
                                color="success"
                                inset
                            ></v-switch>
                            <v-switch
                                v-model="form.versioning_disabled"
                                :label="
                                    t('workflows.create.versioning_disabled') ||
                                    'Disable versioning for this workflow'
                                "
                                color="warning"
                                inset
                            ></v-switch>
                            <v-text-field
                                v-model="form.schedule_cron"
                                label="Cron"
                                :error-messages="cronError || ''"
                                :disabled="hasApiSource"
                                :hint="
                                    hasApiSource
                                        ? t('workflows.create.cron_disabled_for_api_source')
                                        : ''
                                "
                                persistent-hint
                                @update:model-value="onCronChange"
                            />
                            <div
                                v-if="cronHelp && !hasApiSource"
                                class="text-caption mb-2"
                            >
                                {{ cronHelp }}
                            </div>
                            <div
                                v-if="nextRuns.length && !hasApiSource"
                                class="text-caption"
                            >
                                Next: {{ nextRuns.join(', ') }}
                            </div>

                            <v-expansion-panels
                                class="mt-2"
                                :model-value="[]"
                            >
                                <v-expansion-panel>
                                    <v-expansion-panel-title>Config (JSON)</v-expansion-panel-title>
                                    <v-expansion-panel-text>
                                        <div class="mb-4">
                                            <DslConfigurator
                                                v-model="steps"
                                                :workflow-uuid="workflowUuid"
                                            />
                                        </div>
                                        <v-textarea
                                            v-model="configJson"
                                            rows="8"
                                            auto-grow
                                            :error-messages="configError || ''"
                                        />
                                    </v-expansion-panel-text>
                                </v-expansion-panel>
                            </v-expansion-panels>
                        </v-form>
                    </v-window-item>

                    <v-window-item value="history">
                        <div class="mt-4">
                            <div
                                v-if="versions.length === 0"
                                class="text-grey text-body-2"
                            >
                                {{ t('entities.details.no_versions') }}
                            </div>
                            <div v-else>
                                <div class="mb-4">
                                    <div class="text-subtitle-2 mb-2">
                                        Select two versions to compare:
                                    </div>
                                    <v-list
                                        density="compact"
                                        class="version-list"
                                    >
                                        <v-list-item
                                            v-for="version in versions"
                                            :key="version.version_number"
                                            :class="{
                                                'version-selected': isVersionSelected(
                                                    version.version_number
                                                ),
                                                'version-item': true,
                                            }"
                                            @click="toggleVersionSelection(version.version_number)"
                                        >
                                            <template v-slot:prepend>
                                                <v-checkbox
                                                    :model-value="
                                                        isVersionSelected(version.version_number)
                                                    "
                                                    density="compact"
                                                    hide-details
                                                    @click.stop="
                                                        toggleVersionSelection(
                                                            version.version_number
                                                        )
                                                    "
                                                />
                                            </template>
                                            <v-list-item-title>
                                                Version {{ version.version_number }}
                                            </v-list-item-title>
                                            <v-list-item-subtitle>
                                                {{ formatDate(version.created_at) }}
                                                <span v-if="version.created_by_name">
                                                    • {{ version.created_by_name }}
                                                </span>
                                            </v-list-item-subtitle>
                                        </v-list-item>
                                    </v-list>
                                </div>
                                <v-divider class="my-4" />
                                <div
                                    v-if="
                                        diffRows.length === 0 &&
                                        selectedA !== null &&
                                        selectedB !== null
                                    "
                                    class="text-grey text-body-2"
                                >
                                    {{ t('entities.details.no_diff') }}
                                </div>
                                <v-table
                                    v-else-if="diffRows.length > 0"
                                    density="compact"
                                    class="entity-diff-table"
                                >
                                    <thead>
                                        <tr>
                                            <th class="text-left">Field</th>
                                            <th class="text-left">Version {{ selectedA }}</th>
                                            <th class="text-left">Version {{ selectedB }}</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <tr
                                            v-for="row in diffRows"
                                            :key="row.field"
                                            :class="row.changed ? 'changed' : ''"
                                        >
                                            <td class="field">{{ row.field }}</td>
                                            <td class="val">{{ row.a }}</td>
                                            <td class="val">{{ row.b }}</td>
                                        </tr>
                                    </tbody>
                                </v-table>
                            </div>
                        </div>
                    </v-window-item>
                </v-window>
            </v-card-text>
            <v-card-actions>
                <v-spacer />
                <v-btn
                    variant="text"
                    @click="cancel"
                    >Cancel</v-btn
                >
                <v-btn
                    color="primary"
                    :loading="loading"
                    @click="submit"
                    >Save</v-btn
                >
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { computed, ref, watch } from 'vue'
    import { typedHttpClient } from '@/api/typed-client'
    import { ValidationError } from '@/api/typed-client'
    import DslConfigurator from './DslConfigurator.vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { computeDiffRows } from '@/utils/versionDiff'

    const props = defineProps<{ modelValue: boolean; workflowUuid: string | null }>()
    const emit = defineEmits<{
        (e: 'update:modelValue', value: boolean): void
        (e: 'updated'): void
    }>()

    const { t } = useTranslations()
    const model = computed({ get: () => props.modelValue, set: v => emit('update:modelValue', v) })
    const loading = ref(false)
    const formRef = ref()
    const activeTab = ref('edit')

    // Versions/diff
    const versions = ref<
        Array<{
            version_number: number
            created_at: string
            created_by?: string | null
            created_by_name?: string | null
        }>
    >([])
    const selectedA = ref<number | null>(null)
    const selectedB = ref<number | null>(null)
    const diffRows = ref<Array<{ field: string; a: string; b: string; changed: boolean }>>([])

    const kinds = [
        { label: 'Consumer', value: 'consumer' },
        { label: 'Provider', value: 'provider' },
    ]

    const form = ref({
        name: '',
        description: '',
        kind: 'consumer' as 'consumer' | 'provider',
        enabled: true,
        schedule_cron: '' as string | null,
        versioning_disabled: false,
    })

    const configJson = ref('')
    const configError = ref<string | null>(null)
    const steps = ref<any[]>([])
    const cronError = ref<string | null>(null)
    const cronHelp = ref<string>(
        'Use standard 5-field cron (min hour day month dow), e.g. "*/5 * * * *"'
    )
    const nextRuns = ref<string[]>([])
    let cronDebounce: any = null

    // Check if any step has from.api source type (accepts POST, no cron needed)
    const hasApiSource = computed(() => {
        return steps.value.some((step: any) => {
            if (step.from?.type === 'format' && step.from?.source?.source_type === 'api') {
                // from.api without endpoint field = accepts POST
                return !step.from?.source?.config?.endpoint
            }
            return false
        })
    })

    const rules = {
        required: (v: any) => !!v || 'Required',
    }

    watch(
        () => props.modelValue,
        open => {
            if (open) {
                void loadDetails()
                void loadVersions()
            }
        }
    )

    watch(
        () => props.workflowUuid,
        async () => {
            if (props.workflowUuid) {
                await loadVersions()
            }
        },
        { immediate: true }
    )

    const loadVersions = async () => {
        if (!props.workflowUuid) {
            return
        }
        try {
            versions.value = await typedHttpClient.listWorkflowVersions(props.workflowUuid)
            selectedA.value = null
            selectedB.value = null
            diffRows.value = []
        } catch (e) {
            console.error('Failed to load versions:', e)
        }
    }

    const isVersionSelected = (versionNumber: number): boolean => {
        return selectedA.value === versionNumber || selectedB.value === versionNumber
    }

    const toggleVersionSelection = async (versionNumber: number) => {
        if (selectedA.value === versionNumber) {
            // Deselect A
            selectedA.value = selectedB.value
            selectedB.value = null
        } else if (selectedB.value === versionNumber) {
            // Deselect B
            selectedB.value = null
        } else if (selectedA.value === null) {
            // Select as A
            selectedA.value = versionNumber
        } else if (selectedB.value === null) {
            // Select as B
            selectedB.value = versionNumber
            // Auto-load diff when both are selected
            await loadDiff()
        } else {
            // Both are selected, replace A with this version
            selectedA.value = versionNumber
            await loadDiff()
        }
    }

    const loadDiff = async () => {
        diffRows.value = []
        if (!props.workflowUuid || selectedA.value === null || selectedB.value === null) {
            return
        }
        try {
            const [a, b] = await Promise.all([
                typedHttpClient.getWorkflowVersion(props.workflowUuid, selectedA.value),
                typedHttpClient.getWorkflowVersion(props.workflowUuid, selectedB.value),
            ])
            diffRows.value = computeDiffRows(
                (a.data as Record<string, unknown>) ?? {},
                (b.data as Record<string, unknown>) ?? {}
            )
        } catch (e) {
            console.error('Failed to load diff:', e)
        }
    }

    const formatDate = (dateString: string) => {
        return new Date(dateString).toLocaleString()
    }

    async function loadDetails() {
        if (!props.workflowUuid) {
            return
        }
        loading.value = true
        try {
            const data = await typedHttpClient.getWorkflow(props.workflowUuid)
            form.value.name = data.name
            form.value.description = data.description ?? ''
            form.value.kind = data.kind
            form.value.enabled = data.enabled
            form.value.schedule_cron = data.schedule_cron ?? ''
            form.value.versioning_disabled = (data as any).versioning_disabled ?? false
            configJson.value = JSON.stringify(data.config ?? {}, null, 2)
            try {
                const cfg: any = data.config ?? {}
                isSyncingSteps = true
                steps.value = Array.isArray(cfg.steps) ? cfg.steps : []
                // Reset flag after next tick
                setTimeout(() => {
                    isSyncingSteps = false
                }, 0)
            } catch {
                steps.value = []
                isSyncingSteps = false
            }
        } finally {
            loading.value = false
        }
    }

    async function onCronChange(value: string) {
        cronError.value = null
        if (cronDebounce) {
            clearTimeout(cronDebounce)
        }
        if (!value?.trim()) {
            nextRuns.value = []
            return
        }
        cronDebounce = setTimeout(() => {
            void (async () => {
                try {
                    nextRuns.value = await typedHttpClient.previewCron(value)
                } catch {
                    nextRuns.value = []
                }
            })()
        }, 350)
    }

    function cancel() {
        model.value = false
    }

    function parseJson(input: string): any | undefined {
        if (!input?.trim()) {
            return undefined
        }
        try {
            return JSON.parse(input)
        } catch {
            return null
        }
    }

    async function submit() {
        if (!props.workflowUuid) {
            return
        }
        configError.value = null
        cronError.value = null
        // Sync steps to config JSON before validation
        if (steps.value && steps.value.length > 0) {
            isSyncingSteps = true
            configJson.value = JSON.stringify({ steps: steps.value }, null, 2)
            setTimeout(() => {
                isSyncingSteps = false
            }, 0)
        }
        const parsedConfig = parseJson(configJson.value)
        if (parsedConfig === null) {
            configError.value = 'Invalid JSON'
            return
        }
        // Strict DSL presence and validation against BE
        try {
            const steps = Array.isArray(parsedConfig?.steps) ? parsedConfig.steps : null
            if (!steps || steps.length === 0) {
                configError.value = 'DSL steps are required'
                return
            }
            await typedHttpClient.validateDsl(steps)
        } catch (e: any) {
            if (e instanceof ValidationError) {
                // Handle Symfony-style validation errors
                const violations = e.violations || []
                if (violations.length > 0) {
                    // Show all violations, with field names if available
                    const errorMessages = violations.map(v => {
                        const fieldName = v.field && v.field !== 'dsl' ? `${v.field}: ` : ''
                        return `${fieldName}${v.message}`
                    })
                    configError.value = errorMessages.join('; ')
                } else {
                    configError.value = e.message ?? 'Invalid DSL'
                }
                return
            }
            if (e?.violations) {
                const v = e.violations[0]
                configError.value = v?.message ?? 'Invalid DSL'
                return
            }
            configError.value = e instanceof Error ? e.message : 'Invalid DSL'
            return
        }

        loading.value = true
        try {
            await typedHttpClient.updateWorkflow(props.workflowUuid, {
                name: form.value.name,
                description: form.value.description ?? null,
                kind: form.value.kind,
                enabled: form.value.enabled,
                schedule_cron: form.value.schedule_cron ?? null,
                config: parsedConfig ?? {},
                versioning_disabled: form.value.versioning_disabled,
            })
            emit('updated')
            model.value = false
        } catch (e: any) {
            if (e instanceof ValidationError) {
                const cronViolation = e.violations.find(v => v.field === 'schedule_cron')
                if (cronViolation) {
                    cronError.value = cronViolation.message
                }
                return
            }
            throw e
        } finally {
            loading.value = false
        }
    }

    // Sync config JSON when steps change, but only on explicit updates (not during prop sync)
    // We use a flag to prevent recursive updates
    let isSyncingSteps = false
    watch(
        () => steps.value,
        v => {
            if (isSyncingSteps) {
                return
            }
            try {
                const newJson = JSON.stringify({ steps: v }, null, 2)
                // Only update if different to prevent loops
                if (configJson.value !== newJson) {
                    configJson.value = newJson
                }
            } catch {
                // ignore
            }
        },
        { deep: false } // Shallow watch to prevent deep reactivity issues
    )
</script>

<style scoped>
    .version-list {
        max-height: 400px;
        overflow-y: auto;
    }

    .version-item {
        cursor: pointer;
        transition: background-color 0.2s;
    }

    .version-item:hover {
        background-color: rgba(0, 0, 0, 0.04);
    }

    .version-selected {
        background-color: rgba(25, 118, 210, 0.08);
    }

    .entity-diff-table .changed {
        background-color: rgba(255, 193, 7, 0.1);
    }

    .entity-diff-table .field {
        font-weight: 500;
    }

    .entity-diff-table .val {
        font-family: monospace;
        font-size: 0.875rem;
    }
</style>
