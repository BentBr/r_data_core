<template>
    <div class="mt-4">
        <div class="d-flex align-center justify-space-between mb-4">
            <h3 class="text-h6">
                {{ t('entity_definitions.fields.title') }}
            </h3>
            <div class="d-flex">
                <v-btn
                    v-if="hasUnsavedChanges"
                    data-test="apply"
                    color="success"
                    variant="outlined"
                    prepend-icon="mdi-content-save"
                    :loading="savingChanges"
                    class="mr-2"
                    @click="$emit('save-changes')"
                >
                    {{ t('entity_definitions.details.apply_changes') }}
                </v-btn>
                <v-btn
                    data-test="add"
                    color="primary"
                    prepend-icon="mdi-plus"
                    @click="$emit('add-field')"
                >
                    {{ t('entity_definitions.fields.add_field') }}
                </v-btn>
            </div>
        </div>

        <v-treeview
            :items="fieldTreeItems"
            :loading="loading"
            item-key="name"
            activatable
            hoverable
            class="elevation-1"
        >
            <template #prepend="{ item }">
                <v-icon
                    :icon="getFieldIcon(item.field_type)"
                    :color="getFieldColor(item.field_type)"
                    size="small"
                />
            </template>
            <template #title="{ item }">
                <div class="d-flex align-center justify-space-between w-100">
                    <div>
                        <div class="font-weight-medium">
                            {{ item.display_name }}
                        </div>
                        <div class="text-caption text-grey">
                            {{ item.name }}
                        </div>
                    </div>
                    <div class="d-flex align-center">
                        <v-chip
                            size="x-small"
                            color="primary"
                            class="mr-2"
                        >
                            {{ item.field_type }}
                        </v-chip>
                        <v-icon
                            v-if="item.required"
                            icon="mdi-check-circle"
                            color="success"
                            size="small"
                            class="mr-1"
                        />
                        <v-icon
                            v-if="item.indexed"
                            icon="mdi-database"
                            color="info"
                            size="small"
                            class="mr-1"
                        />
                        <v-icon
                            v-if="item.filterable"
                            icon="mdi-filter"
                            color="warning"
                            size="small"
                            class="mr-1"
                        />
                        <v-btn
                            icon="mdi-pencil"
                            size="x-small"
                            variant="text"
                            @click.stop="$emit('edit-field', item)"
                        />
                        <v-btn
                            icon="mdi-delete"
                            size="x-small"
                            variant="text"
                            color="error"
                            @click.stop="$emit('remove-field', item)"
                        />
                    </div>
                </div>
            </template>
        </v-treeview>
    </div>
</template>

<script setup lang="ts">
    import { computed } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import type { EntityDefinition, FieldDefinition } from '@/types/schemas'

    interface Props {
        definition: EntityDefinition
        hasUnsavedChanges: boolean
        savingChanges: boolean
        loading?: boolean
    }

    interface Emits {
        (e: 'save-changes'): void
        (e: 'add-field'): void
        (e: 'edit-field', field: FieldDefinition): void
        (e: 'remove-field', field: FieldDefinition): void
    }

    const props = withDefaults(defineProps<Props>(), {
        loading: false,
    })

    defineEmits<Emits>()
    const { t } = useTranslations()

    const fieldTreeItems = computed(() => {
        return props.definition.fields.map(field => ({
            ...field,
        }))
    })

    const getFieldIcon = (fieldType: string) => {
        const iconMap: Record<string, string> = {
            String: 'mdi-text',
            Text: 'mdi-text-box',
            Wysiwyg: 'mdi-text-box-outline',
            Integer: 'mdi-numeric',
            Float: 'mdi-numeric-1-box',
            Boolean: 'mdi-checkbox-marked',
            Date: 'mdi-calendar',
            DateTime: 'mdi-calendar-clock',
            Object: 'mdi-cube',
            Array: 'mdi-format-list-bulleted',
            Uuid: 'mdi-identifier',
            ManyToOne: 'mdi-link',
            ManyToMany: 'mdi-link-variant',
            Select: 'mdi-format-list-checks',
            MultiSelect: 'mdi-format-list-numbered',
            Image: 'mdi-image',
            File: 'mdi-file',
        }
        return iconMap[fieldType] || 'mdi-text'
    }

    const getFieldColor = (fieldType: string) => {
        const colorMap: Record<string, string> = {
            String: 'primary',
            Text: 'primary',
            Wysiwyg: 'primary',
            Integer: 'success',
            Float: 'success',
            Boolean: 'warning',
            Date: 'info',
            DateTime: 'info',
            Object: 'purple',
            Array: 'orange',
            Uuid: 'grey',
            ManyToOne: 'blue',
            ManyToMany: 'blue',
            Select: 'green',
            MultiSelect: 'green',
            Image: 'pink',
            File: 'brown',
        }
        return colorMap[fieldType] || 'primary'
    }
</script>
