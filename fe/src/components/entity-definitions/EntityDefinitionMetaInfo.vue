<template>
    <v-row class="mt-4">
        <v-col cols="6">
            <v-list>
                <v-list-item>
                    <template #prepend>
                        <v-icon icon="mdi-tag" />
                    </template>
                    <v-list-item-title>{{ t('entity_definitions.meta_info.entity_type') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ definition.entity_type }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item>
                    <template #prepend>
                        <v-icon icon="mdi-text" />
                    </template>
                    <v-list-item-title>{{ t('entity_definitions.meta_info.display_name') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ definition.display_name }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item v-if="definition.description">
                    <template #prepend>
                        <v-icon icon="mdi-information" />
                    </template>
                    <v-list-item-title>{{ t('entity_definitions.meta_info.description') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ definition.description }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item v-if="definition.group_name">
                    <template #prepend>
                        <v-icon icon="mdi-folder" />
                    </template>
                    <v-list-item-title>{{ t('entity_definitions.meta_info.group') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ definition.group_name }}</v-list-item-subtitle>
                </v-list-item>
            </v-list>
        </v-col>
        <v-col cols="6">
            <v-list>
                <v-list-item>
                    <template #prepend>
                        <v-icon icon="mdi-calendar" />
                    </template>
                    <v-list-item-title>{{ t('entity_definitions.meta_info.created') }}</v-list-item-title>
                    <v-list-item-subtitle>{{
                        formatDate(definition.created_at)
                    }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item>
                    <template #prepend>
                        <v-icon icon="mdi-calendar-edit" />
                    </template>
                    <v-list-item-title>{{ t('entity_definitions.meta_info.updated') }}</v-list-item-title>
                    <v-list-item-subtitle>{{
                        formatDate(definition.updated_at)
                    }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item>
                    <template #prepend>
                        <v-icon icon="mdi-counter" />
                    </template>
                    <v-list-item-title>{{ t('entity_definitions.meta_info.version') }}</v-list-item-title>
                    <v-list-item-subtitle>{{ definition.version }}</v-list-item-subtitle>
                </v-list-item>
                <v-list-item>
                    <template #prepend>
                        <v-icon icon="mdi-checkbox-marked-circle" />
                    </template>
                    <v-list-item-title>{{ t('entity_definitions.meta_info.status') }}</v-list-item-title>
                    <v-list-item-subtitle>
                        <v-chip
                            :color="definition.published ? 'success' : 'warning'"
                            size="small"
                        >
                            {{ definition.published ? t('entity_definitions.meta_info.published') : t('entity_definitions.meta_info.draft') }}
                        </v-chip>
                    </v-list-item-subtitle>
                </v-list-item>
            </v-list>
        </v-col>
    </v-row>
</template>

<script setup lang="ts">
    import type { EntityDefinition } from '@/types/schemas'
    import { useTranslations } from '@/composables/useTranslations'

    interface Props {
        definition: EntityDefinition
    }

    defineProps<Props>()

    const { t } = useTranslations()

    const formatDate = (dateString?: string) => {
        if (!dateString) {
            return 'N/A'
        }
        return new Date(dateString).toLocaleDateString()
    }
</script>
