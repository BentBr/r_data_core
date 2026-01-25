<template>
    <v-row>
        <v-col
            cols="12"
            md="3"
        >
            <v-select
                :model-value="permission.resource_type as ResourceNamespace"
                :items="resourceTypes"
                label="Resource Type"
                density="compact"
                :disabled="disabled"
                @update:model-value="updateField('resource_type', $event)"
            />
        </v-col>
        <v-col
            cols="12"
            md="3"
        >
            <v-select
                :model-value="permission.permission_type"
                :items="filteredPermissionTypes"
                label="Permission Type"
                density="compact"
                :disabled="disabled"
                @update:model-value="updateField('permission_type', $event)"
            />
        </v-col>
        <v-col
            cols="12"
            md="3"
        >
            <v-select
                :model-value="permission.access_level"
                :items="accessLevels"
                label="Access Level"
                density="compact"
                :disabled="disabled"
                @update:model-value="updateField('access_level', $event)"
            />
        </v-col>
        <v-col
            cols="12"
            md="3"
        >
            <v-btn
                variant="text"
                color="error"
                size="small"
                :disabled="disabled"
                @click="$emit('remove')"
            >
                <SmartIcon
                    icon="trash-2"
                    :size="16"
                />
            </v-btn>
        </v-col>
    </v-row>
</template>

<script setup lang="ts">
    import { computed, watch } from 'vue'
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import { useTranslations } from '@/composables/useTranslations'
    import type {
        Permission,
        ResourceNamespace,
        PermissionType,
        AccessLevel,
    } from '@/types/schemas'

    const { t } = useTranslations()

    interface Props {
        permission: Permission
        resourceTypes: ResourceNamespace[]
        permissionTypes: PermissionType[]
        accessLevels: AccessLevel[]
        disabled?: boolean
    }

    const props = withDefaults(defineProps<Props>(), {
        disabled: false,
    })

    const emit = defineEmits<{
        update: [permission: Permission]
        remove: []
    }>()

    // Filter permission types: Execute is only available for Workflows
    const filteredPermissionTypes = computed(() => {
        const resourceType = props.permission.resource_type as ResourceNamespace
        const isWorkflows = resourceType === 'Workflows'

        return props.permissionTypes
            .filter(type => {
                // Execute is only available for Workflows namespace
                if (type === 'Execute') {
                    return isWorkflows
                }
                return true
            })
            .map(type => ({
                title: t(`permissions.types.${type.toLowerCase()}`) || type,
                value: type,
            }))
    })

    // Watch for resource type changes and reset permission type if it becomes invalid
    watch(
        () => props.permission.resource_type,
        newResourceType => {
            const resourceType = newResourceType as ResourceNamespace
            const isWorkflows = resourceType === 'Workflows'
            const currentPermissionType = props.permission.permission_type

            // If Execute is selected but resource type is not Workflows, reset to Read
            if (currentPermissionType === 'Execute' && !isWorkflows) {
                emit('update', {
                    ...props.permission,
                    permission_type: 'Read' as PermissionType,
                })
            }
        }
    )

    const updateField = (field: keyof Permission, value: unknown) => {
        emit('update', {
            ...props.permission,
            [field]: value,
        })
    }
</script>
