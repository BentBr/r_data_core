<template>
    <v-row>
        <v-col
            cols="12"
            md="3"
        >
            <v-select
                :model-value="permission.resource_type"
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
                :items="permissionTypes"
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
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import type {
        Permission,
        ResourceNamespace,
        PermissionType,
        AccessLevel,
    } from '@/types/schemas'

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

    const updateField = (field: keyof Permission, value: unknown) => {
        emit('update', {
            ...props.permission,
            [field]: value,
        })
    }
</script>
