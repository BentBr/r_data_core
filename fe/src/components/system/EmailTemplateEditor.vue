<template>
    <v-dialog
        :model-value="modelValue"
        max-width="900px"
        persistent
        @update:model-value="$emit('update:modelValue', $event)"
    >
        <v-card data-testid="email-template-editor">
            <v-card-title class="pa-6">
                {{
                    template ? t('system.email_templates.edit') : t('system.email_templates.create')
                }}
            </v-card-title>

            <v-card-text class="pa-6">
                <v-alert
                    v-if="template?.template_type === 'system'"
                    type="info"
                    variant="tonal"
                    density="compact"
                    class="mb-4"
                >
                    {{ t('system.email_templates.system_template_notice') }}
                </v-alert>

                <v-row>
                    <v-col
                        cols="12"
                        md="8"
                    >
                        <v-text-field
                            v-model="form.name"
                            :label="t('system.email_templates.name')"
                            :readonly="template?.template_type === 'system'"
                            variant="outlined"
                            density="compact"
                            class="mb-3"
                        />
                    </v-col>
                    <v-col
                        cols="12"
                        md="4"
                    >
                        <v-text-field
                            v-model="form.slug"
                            :label="t('system.email_templates.slug')"
                            disabled
                            variant="outlined"
                            density="compact"
                            class="mb-3"
                            :hint="template ? '' : t('system.email_templates.slug_auto_hint')"
                            persistent-hint
                        />
                    </v-col>
                </v-row>

                <v-textarea
                    v-model="form.subject_template"
                    :label="t('system.email_templates.subject')"
                    variant="outlined"
                    density="compact"
                    rows="2"
                    style="font-family: monospace"
                    class="mb-3"
                />

                <v-textarea
                    v-model="form.body_html_template"
                    :label="t('system.email_templates.body_html')"
                    variant="outlined"
                    density="compact"
                    rows="12"
                    style="font-family: monospace"
                    class="mb-3"
                />

                <v-textarea
                    v-model="form.body_text_template"
                    :label="t('system.email_templates.body_text')"
                    variant="outlined"
                    density="compact"
                    rows="6"
                    style="font-family: monospace"
                    class="mb-3"
                />

                <!-- Auto-detected variables -->
                <div class="mt-2">
                    <div class="text-subtitle-2 mb-2">
                        {{ t('system.email_templates.variables') }}
                        <span class="text-caption text-medium-emphasis ml-2">
                            {{ t('system.email_templates.variables_auto_hint') }}
                        </span>
                    </div>

                    <v-alert
                        v-if="detectedVariableKeys.length === 0"
                        type="info"
                        variant="tonal"
                        density="compact"
                        class="mb-2"
                    >
                        {{ t('system.email_templates.no_variables_detected') }}
                    </v-alert>

                    <v-table
                        v-else
                        density="compact"
                    >
                        <thead>
                            <tr>
                                <th style="width: 200px">
                                    {{ t('system.email_templates.variable_key') }}
                                </th>
                                <th>{{ t('system.email_templates.variable_description') }}</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr
                                v-for="key in detectedVariableKeys"
                                :key="key"
                            >
                                <td>
                                    <code v-text="wrapVar(key)" />
                                </td>
                                <td>
                                    <v-text-field
                                        :model-value="variableDescriptions[key] ?? ''"
                                        density="compact"
                                        variant="plain"
                                        hide-details
                                        :placeholder="t('system.email_templates.variable_description_placeholder')"
                                        class="mt-0 pt-0"
                                        @update:model-value="(val: string) => (variableDescriptions[key] = val)"
                                    />
                                </td>
                            </tr>
                        </tbody>
                    </v-table>
                </div>
            </v-card-text>

            <v-card-actions class="pa-4 px-6">
                <v-spacer />
                <v-btn
                    variant="text"
                    color="mutedForeground"
                    @click="$emit('update:modelValue', false)"
                >
                    {{ t('common.cancel') }}
                </v-btn>
                <v-btn
                    color="primary"
                    variant="flat"
                    :loading="saving"
                    @click="handleSave"
                >
                    {{ t('system.email_templates.save') }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { ref, watch, computed } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useSnackbar } from '@/composables/useSnackbar'
    import { useErrorHandler } from '@/composables/useErrorHandler'
    import { typedHttpClient } from '@/api/typed-client'
    import type { EmailTemplate } from '@/api/clients/email-templates'

    interface Props {
        modelValue: boolean
        template: EmailTemplate | null
    }

    interface Emits {
        (e: 'update:modelValue', value: boolean): void
        (e: 'saved'): void
    }

    const props = defineProps<Props>()
    const emit = defineEmits<Emits>()

    const { t } = useTranslations()
    const { showSuccess } = useSnackbar()
    const { handleError } = useErrorHandler()

    const saving = ref(false)

    const form = ref({
        name: '',
        slug: '',
        subject_template: '',
        body_html_template: '',
        body_text_template: '',
    })

    /** Descriptions keyed by variable name — preserved across re-parses */
    const variableDescriptions = ref<Record<string, string>>({})

    /** Extract all unique handlebars variable names from text, ignoring block helpers */
    const extractVariableKeys = (text: string): string[] => {
        const pattern = /\{\{\s*([a-zA-Z_][a-zA-Z0-9_.]*)\s*\}\}/g
        const keys = new Set<string>()
        let m: RegExpExecArray | null = pattern.exec(text)
        while (m !== null) {
            const key = m[1]
            // Skip Handlebars block helpers and keywords
            if (key !== 'else' && key !== 'this') {
                keys.add(key)
            }
            m = pattern.exec(text)
        }
        return [...keys].sort()
    }

    /** All unique variable keys detected from subject + html + text templates */
    const detectedVariableKeys = computed(() => {
        const allText = [
            form.value.subject_template,
            form.value.body_html_template,
            form.value.body_text_template,
        ].join('\n')
        return extractVariableKeys(allText)
    })

    /** Format a variable key as a Handlebars placeholder for display */
    const wrapVar = (key: string): string => `{{${key}}}`

    /** Generate a slug from a display name */
    const toSlug = (name: string): string =>
        name
            .toLowerCase()
            .trim()
            .replace(/\s+/g, '_')
            .replace(/[^a-z0-9_]/g, '')

    // Auto-generate slug from name when creating a new template
    watch(
        () => form.value.name,
        name => {
            if (!props.template) {
                form.value.slug = toSlug(name)
            }
        }
    )

    watch(
        () => props.template,
        template => {
            if (template) {
                form.value = {
                    name: template.name,
                    slug: template.slug,
                    subject_template: template.subject_template,
                    body_html_template: template.body_html_template,
                    body_text_template: template.body_text_template,
                }
                // Restore descriptions from the template's variables JSON
                const vars = template.variables as Array<{ key: string; description: string }>
                const descs: Record<string, string> = {}
                if (Array.isArray(vars)) {
                    for (const v of vars) {
                        descs[v.key] = v.description ?? ''
                    }
                }
                variableDescriptions.value = descs
            } else {
                form.value = {
                    name: '',
                    slug: '',
                    subject_template: '',
                    body_html_template: '',
                    body_text_template: '',
                }
                variableDescriptions.value = {}
            }
        },
        { immediate: true }
    )

    /** Build the variables array from detected keys + user descriptions */
    const buildVariablesPayload = (): Array<{ key: string; description: string }> =>
        detectedVariableKeys.value.map(key => ({
            key,
            description: variableDescriptions.value[key] ?? '',
        }))

    const handleSave = async () => {
        saving.value = true
        try {
            const variables = buildVariablesPayload()
            if (props.template) {
                await typedHttpClient.updateEmailTemplate(props.template.uuid, {
                    name: form.value.name,
                    subject_template: form.value.subject_template,
                    body_html_template: form.value.body_html_template,
                    body_text_template: form.value.body_text_template,
                    variables,
                })
                showSuccess(t('system.email_templates.updated'))
            } else {
                await typedHttpClient.createEmailTemplate({
                    name: form.value.name,
                    slug: form.value.slug,
                    subject_template: form.value.subject_template,
                    body_html_template: form.value.body_html_template,
                    body_text_template: form.value.body_text_template,
                    variables,
                })
                showSuccess(t('system.email_templates.created'))
            }
            emit('update:modelValue', false)
            emit('saved')
        } catch (err) {
            handleError(err)
        } finally {
            saving.value = false
        }
    }
</script>
