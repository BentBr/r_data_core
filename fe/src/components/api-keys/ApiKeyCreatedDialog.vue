<template>
    <v-dialog
        :model-value="modelValue"
        :max-width="getDialogMaxWidth('default')"
        persistent
        @update:model-value="$emit('update:modelValue', $event)"
    >
        <v-card>
            <v-card-title class="d-flex align-center pa-6">
                <SmartIcon
                    icon="check-circle"
                    color="success"
                    size="md"
                    class="mr-2"
                />
                {{ t('api_keys.created.title') }}
            </v-card-title>
            <v-card-text class="pa-6">
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
                            variant="text"
                            size="small"
                            @click="copyApiKey"
                        >
                            <SmartIcon
                                icon="copy"
                                size="sm"
                            />
                        </v-btn>
                    </template>
                </v-text-field>

                <p class="text-body-2 text-medium-emphasis">
                    {{ t('api_keys.created.description') }}
                </p>
            </v-card-text>
            <v-card-actions class="pa-4 px-6">
                <v-spacer />
                <v-btn
                    color="primary"
                    variant="flat"
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
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import { getDialogMaxWidth } from '@/design-system/components'

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

        // Modern approach - all supported browsers have clipboard API
        navigator.clipboard
            .writeText(apiKey)
            .then(() => {
                emit('copy-success')
            })
            .catch(err => {
                console.error('Failed to copy API key:', err)
            })
    }
</script>
