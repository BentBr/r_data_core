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
                    :loading="savingChanges"
                    class="mr-2"
                    @click="$emit('save-changes')"
                >
                    <template #prepend>
                        <SmartIcon
                            icon="save"
                            :size="20"
                        />
                    </template>
                    {{ t('entity_definitions.details.apply_changes') }}
                </v-btn>
                <v-btn
                    data-test="add"
                    color="primary"
                    @click="$emit('add-field')"
                >
                    <template #prepend>
                        <SmartIcon
                            icon="plus"
                            :size="20"
                        />
                    </template>
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
                <SmartIcon
                    :icon="getFieldIcon(item.field_type)"
                    :color="getFieldColor(item.field_type)"
                    :size="20"
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
                        <Badge
                            color="primary"
                            size="small"
                            class="mr-2"
                        >
                            {{ getFieldTypeDisplayName(item.field_type) }}
                        </Badge>
                        <v-tooltip
                            v-if="item.required"
                            :text="t('entity_definitions.fields.required')"
                            location="top"
                        >
                            <template #activator="{ props: tooltipProps }">
                                <span v-bind="tooltipProps">
                                    <SmartIcon
                                        icon="check-circle"
                                        size="sm"
                                        class="mr-1 text-success"
                                    />
                                </span>
                            </template>
                        </v-tooltip>
                        <v-tooltip
                            v-if="item.indexed"
                            :text="t('entity_definitions.fields.indexed')"
                            location="top"
                        >
                            <template #activator="{ props: tooltipProps }">
                                <span v-bind="tooltipProps">
                                    <SmartIcon
                                        icon="database"
                                        :size="20"
                                        class="mr-1 text-info"
                                    />
                                </span>
                            </template>
                        </v-tooltip>
                        <v-tooltip
                            v-if="item.unique"
                            :text="t('entity_definitions.fields.unique')"
                            location="top"
                        >
                            <template #activator="{ props: tooltipProps }">
                                <span v-bind="tooltipProps">
                                    <SmartIcon
                                        icon="key"
                                        size="sm"
                                        class="mr-1 text-purple"
                                    />
                                </span>
                            </template>
                        </v-tooltip>
                        <v-tooltip
                            v-if="item.filterable"
                            :text="t('entity_definitions.fields.filterable')"
                            location="top"
                        >
                            <template #activator="{ props: tooltipProps }">
                                <span v-bind="tooltipProps">
                                    <SmartIcon
                                        icon="filter"
                                        size="sm"
                                        class="mr-1 text-warning"
                                    />
                                </span>
                            </template>
                        </v-tooltip>
                        <v-tooltip
                            v-if="item.constraints?.constraints?.pattern"
                            :text="t('entity_definitions.fields.pattern')"
                            location="top"
                        >
                            <template #activator="{ props: tooltipProps }">
                                <span v-bind="tooltipProps">
                                    <SmartIcon
                                        icon="regex"
                                        size="sm"
                                        class="mr-1 text-cyan"
                                    />
                                </span>
                            </template>
                        </v-tooltip>
                        <v-btn
                            size="x-small"
                            variant="text"
                            @click.stop="$emit('edit-field', item)"
                        >
                            <SmartIcon
                                icon="pencil"
                                size="xs"
                            />
                        </v-btn>
                        <v-btn
                            size="x-small"
                            variant="text"
                            color="error"
                            @click.stop="$emit('remove-field', item)"
                        >
                            <SmartIcon
                                icon="trash-2"
                                size="xs"
                            />
                        </v-btn>
                    </div>
                </div>
            </template>
        </v-treeview>
    </div>
</template>

<script setup lang="ts">
    import { computed } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import Badge from '@/components/common/Badge.vue'
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
            String: 'type',
            Text: 'file-text',
            Wysiwyg: 'file-edit',
            Integer: 'hash',
            Float: 'hash',
            Boolean: 'check-square',
            Date: 'calendar',
            DateTime: 'calendar-clock',
            Object: 'box',
            Array: 'list',
            Json: 'braces',
            Uuid: 'hash',
            ManyToOne: 'link',
            ManyToMany: 'link-2',
            Select: 'list-checks',
            MultiSelect: 'list-checks',
            Image: 'image',
            File: 'file',
        }
        return iconMap[fieldType] || 'type'
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
            Json: 'teal',
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

    const getFieldTypeDisplayName = (fieldType: string) => {
        const displayNameMap: Record<string, string> = {
            Object: 'Json Object',
            Array: 'Json Array',
            Json: 'Json (any)',
        }
        return displayNameMap[fieldType] || fieldType
    }
</script>
