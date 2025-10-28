<template>
    <v-dialog
        :model-value="modelValue"
        max-width="600px"
        persistent
        @update:model-value="$emit('update:modelValue', $event)"
    >
        <v-card>
            <v-card-title class="d-flex align-center">
                <v-icon
                    icon="mdi-check-circle"
                    color="success"
                    class="mr-2"
                />
                {{ t('api_keys.created.title') }}
            </v-card-title>
            <v-card-text>
                <v-alert
                    type="warning"
                    variant="tonal"
                    class="mb-4"
                >
                    {{ t('api_keys.created.warning') }}
                </v-alert>

                <v-text-field
                    id="apiKey"
                    :model-value="apiKey"
                    :label="t('api_keys.created.key_label')"
                    readonly
                    variant="outlined"
                    class="mb-4"
                >
                    <template #append>
                        <v-btn
                            icon="mdi-content-copy"
                            variant="text"
                            size="small"
                            @click="copyApiKey"
                        />
                    </template>
                </v-text-field>

                <p class="text-body-2 text-medium-emphasis">
                    {{ t('api_keys.created.description') }}
                </p>
            </v-card-text>
            <v-card-actions>
                <v-spacer />
                <v-btn
                    color="primary"
                    @click="$emit('update:modelValue', false)"
                >
                    {{ t('common.close') }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { useTranslations } from '@/composables/useTranslations'

    const { t } = useTranslations()

    // Props
    interface Props {
        modelValue: boolean
        apiKey: string
    }

    const props = defineProps<Props>()

    // Emits
    const emit = defineEmits<{
        'update:modelValue': [value: boolean]
        'copy-success': []
    }>()

    // Methods
    const copyApiKey = () => {
        const apiKey = props.apiKey

        // Modern approach
        if (navigator?.clipboard?.writeText) {
            navigator.clipboard
                .writeText(apiKey)
                .then(() => {
                    emit('copy-success')
                })
                .catch(err => {
                    console.error('Failed to copy API key:', err)
                })
        } else {
            // Fallback method (old browsers / JS) - based on masteringjs.io tutorial
            const input = document.querySelector('#apiKey') as HTMLInputElement
            if (input) {
                input.select()
                input.setSelectionRange(0, 99999)
                try {
                    document.execCommand('copy')
                    emit('copy-success')
                } catch (err) {
                    console.error('Failed to copy API key:', err)
                }
            }
        }
    }
</script>
