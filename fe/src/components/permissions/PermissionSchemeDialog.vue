<template>
    <v-dialog
        :model-value="modelValue"
        max-width="900px"
        persistent
        @update:model-value="$emit('update:modelValue', $event)"
    >
        <v-card>
            <v-card-title>
                {{ editingScheme ? 'Edit Permission Scheme' : 'Create Permission Scheme' }}
            </v-card-title>
            <v-card-text>
                <v-form
                    ref="formRef"
                    v-model="formValid"
                >
                    <v-text-field
                        v-model="formData.name"
                        label="Name"
                        :rules="[rules.required]"
                        required
                    />
                    <v-textarea
                        v-model="formData.description"
                        label="Description"
                        rows="2"
                    />

                    <v-divider class="my-4" />

                    <div class="text-h6 mb-4">Roles & Permissions</div>

                    <RolePermissionsEditor
                        v-for="(permissions, roleName, index) in formData.role_permissions"
                        :key="index"
                        :role-name="roleNames[index]"
                        :permissions="permissions"
                        :resource-types="resourceTypes"
                        :permission-types="permissionTypes"
                        :access-levels="accessLevels"
                        class="mb-6"
                        @update:role-name="updateRoleName(index, $event)"
                        @update:permissions="updateRolePermissions(roleName, $event)"
                        @add-permission="addPermission(index)"
                        @remove="removeRole(index)"
                    />

                    <v-btn
                        color="primary"
                        variant="outlined"
                        prepend-icon="mdi-plus"
                        @click="addRole"
                    >
                        Add Role
                    </v-btn>
                </v-form>
            </v-card-text>
            <v-card-actions>
                <v-spacer />
                <v-btn
                    variant="text"
                    @click="handleClose"
                >
                    Cancel
                </v-btn>
                <v-btn
                    color="primary"
                    :loading="loading"
                    :disabled="!formValid"
                    @click="handleSave"
                >
                    {{ editingScheme ? 'Update' : 'Create' }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { ref, watch } from 'vue'
    import RolePermissionsEditor from './RolePermissionsEditor.vue'
    import type {
        PermissionScheme,
        CreatePermissionSchemeRequest,
        UpdatePermissionSchemeRequest,
        Permission,
        ResourceNamespace,
        PermissionType,
        AccessLevel,
    } from '@/types/schemas'

    interface Props {
        modelValue: boolean
        editingScheme?: PermissionScheme | null
        loading?: boolean
        resourceTypes: ResourceNamespace[]
        permissionTypes: PermissionType[]
        accessLevels: AccessLevel[]
    }

    interface Emits {
        (e: 'update:modelValue', value: boolean): void
        (e: 'save', data: CreatePermissionSchemeRequest | UpdatePermissionSchemeRequest): void
    }

    const props = withDefaults(defineProps<Props>(), {
        editingScheme: null,
        loading: false,
    })

    const emit = defineEmits<Emits>()

    const formRef = ref()
    const formValid = ref(false)

    const formData = ref<{
        name: string
        description: string | null
        role_permissions: Record<string, Permission[]>
    }>({
        name: '',
        description: null,
        role_permissions: {},
    })

    const roleNames = ref<string[]>([])

    const rules = {
        required: (v: string) => !!v || 'Required',
    }

    // Reset form when dialog opens/closes or editingScheme changes
    watch(
        () => [props.modelValue, props.editingScheme],
        () => {
            if (props.modelValue) {
                if (props.editingScheme) {
                    formData.value = {
                        name: props.editingScheme.name,
                        description: props.editingScheme.description ?? null,
                        role_permissions: JSON.parse(
                            JSON.stringify(props.editingScheme.role_permissions)
                        ),
                    }
                    roleNames.value = Object.keys(formData.value.role_permissions)
                } else {
                    formData.value = {
                        name: '',
                        description: null,
                        role_permissions: {},
                    }
                    roleNames.value = []
                }
            }
        },
        { immediate: true }
    )

    const addRole = () => {
        const newRoleName = `Role${Object.keys(formData.value.role_permissions).length + 1}`
        formData.value.role_permissions[newRoleName] = []
        roleNames.value.push(newRoleName)
    }

    const removeRole = (index: number) => {
        const roleName = roleNames.value[index]
        delete formData.value.role_permissions[roleName]
        roleNames.value.splice(index, 1)
    }

    const updateRoleName = (index: number, newName: string) => {
        roleNames.value[index] = newName
    }

    const updateRolePermissions = (oldRoleName: string, permissions: Permission[]) => {
        formData.value.role_permissions[oldRoleName] = permissions
    }

    const addPermission = (roleIndex: number) => {
        const roleName = roleNames.value[roleIndex]
        if (!formData.value.role_permissions[roleName]) {
            formData.value.role_permissions[roleName] = []
        }
        formData.value.role_permissions[roleName].push({
            resource_type: 'Workflows',
            permission_type: 'Read',
            access_level: 'All',
            resource_uuids: [],
            constraints: null,
        })
    }

    const handleClose = () => {
        emit('update:modelValue', false)
    }

    const handleSave = () => {
        if (!formValid.value) {
            return
        }

        // Update role names
        const updatedRolePermissions: Record<string, Permission[]> = {}
        roleNames.value.forEach((newName, index) => {
            const oldName = Object.keys(formData.value.role_permissions)[index]
            updatedRolePermissions[newName] = formData.value.role_permissions[oldName] || []
        })

        const requestData = {
            name: formData.value.name,
            description: formData.value.description,
            role_permissions: updatedRolePermissions,
        }

        emit('save', requestData)
    }
</script>
