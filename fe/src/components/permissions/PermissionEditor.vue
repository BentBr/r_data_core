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
                @update:model-value="updateField('access_level', $event)"
            />
        </v-col>
        <v-col
            cols="12"
            md="3"
        >
            <v-btn
                icon="mdi-delete"
                variant="text"
                color="error"
                size="small"
                @click="$emit('remove')"
            />
        </v-col>
    </v-row>
</template>

<script setup lang="ts">
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
    }

    const props = defineProps<Props>()

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
