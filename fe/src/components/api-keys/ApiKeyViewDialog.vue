<template>
    <v-dialog
        :model-value="modelValue"
        :max-width="getDialogMaxWidth('default')"
        @update:model-value="$emit('update:modelValue', $event)"
    >
        <v-card>
            <v-card-title class="pa-6">{{ t('api_keys.view.title') }}</v-card-title>
            <v-card-text class="pa-6">
                <div v-if="apiKey">
                    <v-list>
                        <v-list-item>
                            <template #prepend>
                                <div class="mr-3">
                                    <SmartIcon
                                        icon="key"
                                        size="sm"
                                    />
                                </div>
                            </template>
                            <v-list-item-title>{{ t('api_keys.view.name') }}</v-list-item-title>
                            <v-list-item-subtitle>{{ apiKey.name }}</v-list-item-subtitle>
                        </v-list-item>
                        <v-list-item v-if="apiKey.description">
                            <template #prepend>
                                <div class="mr-3">
                                    <SmartIcon
                                        icon="type"
                                        size="sm"
                                    />
                                </div>
                            </template>
                            <v-list-item-title>{{
                                t('api_keys.view.description')
                            }}</v-list-item-title>
                            <v-list-item-subtitle>{{ apiKey.description }}</v-list-item-subtitle>
                        </v-list-item>
                        <v-list-item>
                            <template #prepend>
                                <div class="mr-3">
                                    <SmartIcon
                                        icon="calendar"
                                        size="sm"
                                    />
                                </div>
                            </template>
                            <v-list-item-title>{{ t('api_keys.view.created') }}</v-list-item-title>
                            <v-list-item-subtitle>{{
                                formatDate(apiKey.created_at)
                            }}</v-list-item-subtitle>
                        </v-list-item>
                        <v-list-item v-if="apiKey.expires_at">
                            <template #prepend>
                                <div class="mr-3">
                                    <SmartIcon
                                        icon="calendar-clock"
                                        size="sm"
                                    />
                                </div>
                            </template>
                            <v-list-item-title>{{ t('api_keys.view.expires') }}</v-list-item-title>
                            <v-list-item-subtitle>{{
                                formatDate(apiKey.expires_at)
                            }}</v-list-item-subtitle>
                        </v-list-item>
                        <v-list-item v-if="apiKey.last_used_at">
                            <template #prepend>
                                <div class="mr-3">
                                    <SmartIcon
                                        icon="clock"
                                        size="sm"
                                    />
                                </div>
                            </template>
                            <v-list-item-title>{{
                                t('api_keys.view.last_used')
                            }}</v-list-item-title>
                            <v-list-item-subtitle>{{
                                formatDate(apiKey.last_used_at)
                            }}</v-list-item-subtitle>
                        </v-list-item>
                    </v-list>
                </div>
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
    import type { ApiKey } from '@/types/schemas'

    const { t } = useTranslations()

    // Props
    interface Props {
        modelValue: boolean
        apiKey: ApiKey | null
    }

    defineProps<Props>()

    // Emits
    defineEmits<{
        'update:modelValue': [value: boolean]
    }>()

    // Methods
    const formatDate = (dateString: string | null): string => {
        if (!dateString) {
            return 'Never'
        }
        return new Date(dateString).toLocaleString()
    }
</script>
