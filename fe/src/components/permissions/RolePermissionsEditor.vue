<template>
    <v-card
        variant="outlined"
        class="pa-4"
    >
        <div class="d-flex align-center justify-space-between mb-3">
            <v-text-field
                :model-value="roleName"
                label="Role Name"
                density="compact"
                class="mr-2"
                @update:model-value="$emit('update:roleName', $event)"
            />
            <v-btn
                variant="text"
                color="error"
                size="small"
                @click="$emit('remove')"
            >
                <SmartIcon
                    icon="trash-2"
                    :size="16"
                />
            </v-btn>
        </div>

        <div
            v-for="(permission, permIndex) in permissions"
            :key="permIndex"
            class="mb-3"
        >
            <PermissionEditor
                :permission="permission"
                :resource-types="resourceTypes"
                :permission-types="permissionTypes"
                :access-levels="accessLevels"
                @update="updatePermission(permIndex, $event)"
                @remove="removePermission(permIndex)"
            />
        </div>

        <v-btn
            color="primary"
            variant="outlined"
            size="small"
            @click="$emit('add-permission')"
        >
            <template #prepend>
                <SmartIcon
                    icon="plus"
                    :size="20"
                />
            </template>
            Add Permission
        </v-btn>
    </v-card>
</template>

<script setup lang="ts">
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import PermissionEditor from './PermissionEditor.vue'
    import type {
        Permission,
        ResourceNamespace,
        PermissionType,
        AccessLevel,
    } from '@/types/schemas'

    interface Props {
        roleName: string
        permissions: Permission[]
        resourceTypes: ResourceNamespace[]
        permissionTypes: PermissionType[]
        accessLevels: AccessLevel[]
    }

    const props = defineProps<Props>()

    const emit = defineEmits<{
        'update:roleName': [name: string]
        'update:permissions': [permissions: Permission[]]
        'add-permission': []
        remove: []
    }>()

    const updatePermission = (index: number, permission: Permission) => {
        const updated = [...props.permissions]
        updated[index] = permission
        emit('update:permissions', updated)
    }

    const removePermission = (index: number) => {
        const updated = [...props.permissions]
        updated.splice(index, 1)
        emit('update:permissions', updated)
    }
</script>
