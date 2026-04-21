<template>
    <v-expansion-panels
        variant="accordion"
        class="mt-4"
    >
        <v-expansion-panel>
            <v-expansion-panel-title>
                {{ t('workflows.dsl.post_run.title') }}
                <v-chip
                    v-if="actions.length > 0"
                    size="x-small"
                    class="ml-2"
                >
                    {{ actions.length }}
                </v-chip>
            </v-expansion-panel-title>
            <v-expansion-panel-text>
                <v-alert
                    type="info"
                    variant="tonal"
                    density="compact"
                    class="mb-3"
                >
                    {{ t('workflows.dsl.post_run.info') }}
                </v-alert>

                <!-- List of actions -->
                <div
                    v-for="(action, idx) in actions"
                    :key="idx"
                    class="mb-4 pa-3 border rounded"
                >
                    <div class="d-flex justify-space-between align-center mb-2">
                        <span class="text-subtitle-2">
                            {{ t('workflows.dsl.post_run.action') }} {{ idx + 1 }}:
                            {{ t('workflows.dsl.post_run.send_email') }}
                        </span>
                        <v-btn
                            icon
                            size="small"
                            variant="text"
                            @click="removeAction(idx)"
                        >
                            <v-icon>mdi-close</v-icon>
                        </v-btn>
                    </div>

                    <!-- Template selector -->
                    <v-select
                        :model-value="action.template_uuid"
                        :items="emailTemplateItems"
                        item-title="title"
                        item-value="value"
                        :label="t('workflows.dsl.send_email_template')"
                        density="comfortable"
                        class="mb-2"
                        :hint="t('workflows.dsl.hints.send_email.template_uuid')"
                        persistent-hint
                        @update:model-value="(v: string) => onTemplateSelected(idx, v)"
                    />

                    <!-- Show template variables when selected -->
                    <v-alert
                        v-if="getTemplateVars(action.template_uuid).length > 0"
                        type="info"
                        variant="tonal"
                        density="compact"
                        class="mb-3"
                    >
                        <div class="text-caption font-weight-bold mb-1">
                            {{ t('workflows.dsl.post_run.available_run_vars') }}
                        </div>
                        <div class="text-caption">
                            <code v-text="runContextVars" />
                        </div>
                        <div class="text-caption mt-1 font-weight-bold">
                            {{ t('workflows.dsl.hints.send_email.template_variables') }}
                        </div>
                        <div
                            v-for="v in getTemplateVars(action.template_uuid)"
                            :key="v.key"
                            class="text-caption"
                        >
                            <code v-text="wrapVar(v.key)" /> —
                            {{ v.description || '(no description)' }}
                        </div>
                    </v-alert>

                    <!-- Recipients -->
                    <div class="text-caption mb-1">{{ t('workflows.dsl.send_email_to') }}</div>
                    <div
                        v-for="(recipient, rIdx) in action.to"
                        :key="`to-${rIdx}`"
                        class="d-flex ga-2 mb-2 align-center"
                    >
                        <v-text-field
                            :model-value="recipient.kind === 'const_string' ? recipient.value : ''"
                            label="Email address"
                            density="comfortable"
                            @update:model-value="(v: string) => updateRecipient(idx, rIdx, v)"
                        />
                        <v-btn
                            icon
                            size="small"
                            variant="text"
                            @click="removeRecipient(idx, rIdx)"
                        >
                            <v-icon>mdi-close</v-icon>
                        </v-btn>
                    </div>
                    <v-btn
                        variant="outlined"
                        size="small"
                        class="mb-3"
                        @click="addRecipient(idx)"
                    >
                        {{ t('workflows.dsl.add_recipient') }}
                    </v-btn>

                    <!-- Condition -->
                    <v-select
                        :model-value="action.condition ?? 'always'"
                        :items="conditionOptions"
                        item-title="title"
                        item-value="value"
                        :label="t('workflows.dsl.post_run.condition')"
                        density="comfortable"
                        :hint="t('workflows.dsl.post_run.condition_hint')"
                        persistent-hint
                        @update:model-value="(v: string) => updateAction(idx, 'condition', v)"
                    />
                </div>

                <!-- Add action button -->
                <v-btn
                    variant="outlined"
                    size="small"
                    @click="addEmailAction"
                >
                    {{ t('workflows.dsl.post_run.add_email_action') }}
                </v-btn>
            </v-expansion-panel-text>
        </v-expansion-panel>
    </v-expansion-panels>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { typedHttpClient } from '@/api/typed-client'
    import type { EmailTemplate } from '@/api/clients/email-templates'
    import type { OnComplete, PostRunAction } from '@/types/schemas/dsl'

    const props = defineProps<{
        modelValue: OnComplete | null | undefined
    }>()

    const emit = defineEmits<{
        (e: 'update:modelValue', value: OnComplete | null): void
    }>()

    const { t } = useTranslations()

    const emailTemplates = ref<EmailTemplate[]>([])
    const emailTemplateItems = ref<{ title: string; value: string }[]>([])

    // Run context variable display — using v-text to avoid Vue interpolation of {{ }}
    const runContextVars = computed(
        () =>
            '{{run.workflow_name}}, {{run.processed_items}}, {{run.failed_items}}, {{run.total_items}}, {{run.status}}, {{run.duration_seconds}}'
    )

    const conditionOptions = [
        { title: 'Always', value: 'always' },
        { title: 'On Success (all items passed)', value: 'on_success' },
        { title: 'On Failure (at least one item failed)', value: 'on_failure' },
    ]

    const actions = computed((): PostRunAction[] => props.modelValue?.actions ?? [])

    const wrapVar = (key: string): string => `{{${key}}}`

    function getTemplateVars(uuid: string): Array<{ key: string; description: string }> {
        const tmpl = emailTemplates.value.find(t => t.uuid === uuid)
        if (!tmpl) {
            return []
        }
        const vars = tmpl.variables as Array<{ key: string; description: string }>
        return Array.isArray(vars) ? vars : []
    }

    onMounted(() => {
        void loadEmailTemplates()
    })

    async function loadEmailTemplates() {
        try {
            const templates = await typedHttpClient.listEmailTemplates('workflow')
            emailTemplates.value = templates
            emailTemplateItems.value = templates.map(tmpl => ({
                title: tmpl.name,
                value: tmpl.uuid,
            }))
        } catch {
            emailTemplates.value = []
            emailTemplateItems.value = []
        }
    }

    function emitUpdated(newActions: PostRunAction[]) {
        if (newActions.length === 0) {
            emit('update:modelValue', null)
        } else {
            emit('update:modelValue', { actions: newActions })
        }
    }

    function addEmailAction() {
        const newAction: PostRunAction = {
            type: 'send_email',
            template_uuid: '',
            to: [],
            cc: null,
            condition: 'always',
        }
        emitUpdated([...actions.value, newAction])
    }

    function removeAction(idx: number) {
        const updated = actions.value.filter((_, i) => i !== idx)
        emitUpdated(updated)
    }

    function updateAction(idx: number, field: string, value: unknown) {
        const updated = actions.value.map((a, i) => {
            if (i !== idx) {
                return a
            }
            return { ...a, [field]: value } as PostRunAction
        })
        emitUpdated(updated)
    }

    function onTemplateSelected(idx: number, uuid: string) {
        updateAction(idx, 'template_uuid', uuid)
    }

    function addRecipient(actionIdx: number) {
        const updated = actions.value.map((action, index) => {
            if (index !== actionIdx) {
                return action
            }
            const existing = action.to
            return {
                ...action,
                to: [...existing, { kind: 'const_string' as const, value: '' }],
            } as PostRunAction
        })
        emitUpdated(updated)
    }

    function removeRecipient(actionIdx: number, recipientIdx: number) {
        const updated = actions.value.map((action, index) => {
            if (index !== actionIdx) {
                return action
            }
            return {
                ...action,
                to: action.to.filter((_, ri) => ri !== recipientIdx),
            } as PostRunAction
        })
        emitUpdated(updated)
    }

    function updateRecipient(actionIdx: number, recipientIdx: number, value: string) {
        const updated = actions.value.map((action, index) => {
            if (index !== actionIdx) {
                return action
            }
            const newTo = action.to.map((r, ri) => {
                if (ri !== recipientIdx) {
                    return r
                }
                return { kind: 'const_string' as const, value }
            })
            return { ...action, to: newTo } as PostRunAction
        })
        emitUpdated(updated)
    }
</script>
