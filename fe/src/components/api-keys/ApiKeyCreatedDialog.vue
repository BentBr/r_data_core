<template>
    <v-dialog
        v-model="model"
        :max-width="getDialogMaxWidth('default')"
        persistent
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
                    @click="model = false"
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
    import { copyText } from '@/utils/clipboard'

    const { t } = useTranslations()

    // Props
    interface Props {
        apiKey: string
    }
    const model = defineModel<boolean>({ required: true })

    const props = defineProps<Props>()

    // Emits
    const emit = defineEmits<{
        'copy-success': []
        'copy-failure': []
    }>()

    // Methods
    const copyApiKey = async () => {
        // navigator.clipboard is undefined outside a secure context, and the
        // admin runs on plain HTTP locally. Reading .writeText off undefined
        // threw synchronously, so the old .catch() never ran and the button
        // failed silently. copyText falls back to execCommand.
        const copied = await copyText(props.apiKey)

        if (copied) {
            emit('copy-success')
        } else {
            emit('copy-failure')
        }
    }
</script>
