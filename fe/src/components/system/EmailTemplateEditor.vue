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
                            readonly
                            variant="outlined"
                            density="compact"
                            class="mb-3"
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

                <div
                    v-if="form.variables.length > 0"
                    class="mt-2"
                >
                    <div class="text-subtitle-2 mb-2">
                        {{ t('system.email_templates.variables') }}
                    </div>
                    <v-table density="compact">
                        <thead>
                            <tr>
                                <th>{{ t('system.email_templates.variable_key') }}</th>
                                <th>{{ t('system.email_templates.variable_description') }}</th>
                            </tr>
                        </thead>
                        <tbody>
                            <tr
                                v-for="variable in form.variables"
                                :key="variable.key"
                            >
                                <td>
                                    <code>{{ variable.key }}</code>
                                </td>
                                <td>{{ variable.description }}</td>
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
    import { ref, watch } from 'vue'
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
        variables: [] as Array<{ key: string; description: string }>,
    })

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
                    variables: [...(template.variables as Array<{ key: string; description: string }>)],
                }
            } else {
                form.value = {
                    name: '',
                    slug: '',
                    subject_template: '',
                    body_html_template: '',
                    body_text_template: '',
                    variables: [],
                }
            }
        },
        { immediate: true }
    )

    const handleSave = async () => {
        saving.value = true
        try {
            if (props.template) {
                await typedHttpClient.updateEmailTemplate(props.template.uuid, {
                    name: form.value.name,
                    subject_template: form.value.subject_template,
                    body_html_template: form.value.body_html_template,
                    body_text_template: form.value.body_text_template,
                    variables: form.value.variables,
                })
                showSuccess(t('system.email_templates.updated'))
            } else {
                await typedHttpClient.createEmailTemplate({
                    name: form.value.name,
                    slug: form.value.slug,
                    subject_template: form.value.subject_template,
                    body_html_template: form.value.body_html_template,
                    body_text_template: form.value.body_text_template,
                    variables: form.value.variables,
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
