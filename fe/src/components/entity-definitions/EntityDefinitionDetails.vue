<template>
    <v-card
        v-if="definition"
        variant="outlined"
    >
        <v-card-title class="d-flex align-center justify-space-between pa-4">
            <div class="d-flex align-center">
                <v-icon
                    :icon="definition.icon || 'mdi-file-document'"
                    class="mr-3"
                />
                <span class="text-h5">{{ definition.display_name }}</span>
            </div>
            <div>
                <v-btn
                    color="primary"
                    variant="outlined"
                    prepend-icon="mdi-pencil"
                    class="mr-2"
                    @click="$emit('edit')"
                >
                    Edit
                </v-btn>

                <v-btn
                    color="error"
                    variant="outlined"
                    prepend-icon="mdi-delete"
                    @click="$emit('delete')"
                >
                    {{ t('entity_definitions.delete.button') }}
                </v-btn>
            </div>
        </v-card-title>

        <v-card-text>
            <v-tabs v-model="activeTab">
                <v-tab value="meta">{{ t('entity_definitions.details.meta_info') }}</v-tab>
                <v-tab value="fields">{{ t('entity_definitions.details.fields') }}</v-tab>
            </v-tabs>

            <v-window v-model="activeTab">
                <!-- Meta Information Tab -->
                <v-window-item value="meta">
                    <EntityDefinitionMetaInfo :definition="definition" />
                </v-window-item>

                <!-- Fields Tab -->
                <v-window-item value="fields">
                    <EntityDefinitionFields
                        :definition="definition"
                        :has-unsaved-changes="hasUnsavedChanges"
                        :saving-changes="savingChanges"
                        @save-changes="$emit('save-changes')"
                        @add-field="$emit('add-field')"
                        @edit-field="$emit('edit-field', $event)"
                        @remove-field="$emit('remove-field', $event)"
                    />
                </v-window-item>
            </v-window>
        </v-card-text>
    </v-card>

    <v-card
        v-else
        variant="outlined"
    >
        <v-card-text class="text-center pa-8">
            <v-icon
                icon="mdi-file-document-outline"
                size="64"
                color="grey"
                class="mb-4"
            />
            <h3 class="text-h6 text-grey">
                {{ t('entity_definitions.details.select_entity') }}
            </h3>
            <p class="text-body-2 text-grey">
                {{ t('entity_definitions.details.select_entity_description') }}
            </p>
        </v-card-text>
    </v-card>
</template>

<script setup lang="ts">
    import { ref } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import EntityDefinitionMetaInfo from './EntityDefinitionMetaInfo.vue'
    import EntityDefinitionFields from './EntityDefinitionFields.vue'
    import type { EntityDefinition } from '@/types/schemas'

    interface Props {
        definition: EntityDefinition | null
        hasUnsavedChanges: boolean
        savingChanges: boolean
    }

    interface Emits {
        (e: 'edit'): void
        (e: 'delete'): void
        (e: 'save-changes'): void
        (e: 'add-field'): void
        (e: 'edit-field', field: import('@/types/schemas').FieldDefinition): void
        (e: 'remove-field', field: import('@/types/schemas').FieldDefinition): void
    }

    defineProps<Props>()
    defineEmits<Emits>()
    const { t } = useTranslations()

    const activeTab = ref('meta')
</script>
