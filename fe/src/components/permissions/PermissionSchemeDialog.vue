<template>
    <v-dialog
        :model-value="modelValue"
        max-width="900px"
        persistent
        @update:model-value="$emit('update:modelValue', $event)"
    >
        <v-card>
            <v-card-title>
                {{
                    editingScheme
                        ? t('permissions.dialog.edit_title')
                        : t('permissions.dialog.create_title')
                }}
            </v-card-title>
            <v-card-text>
                <v-form
                    ref="formRef"
                    v-model="formValid"
                >
                    <v-text-field
                        v-model="formData.name"
                        :label="t('permissions.dialog.name')"
                        :rules="[rules.required]"
                        required
                    />
                    <v-textarea
                        v-model="formData.description"
                        :label="t('permissions.dialog.description')"
                        rows="2"
                    />

                    <v-checkbox
                        v-model="formData.super_admin"
                        :label="t('permissions.dialog.super_admin')"
                        :hint="t('permissions.dialog.super_admin_hint')"
                        persistent-hint
                    />

                    <v-divider class="my-4" />

                    <div class="text-h6 mb-4">{{ t('permissions.dialog.roles_permissions') }}</div>

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
                        {{ t('permissions.dialog.add_role') }}
                    </v-btn>
                </v-form>
            </v-card-text>
            <v-card-actions>
                <v-spacer />
                <v-btn
                    variant="text"
                    @click="handleClose"
                >
                    {{ t('permissions.dialog.cancel') }}
                </v-btn>
                <v-btn
                    color="primary"
                    :loading="loading"
                    :disabled="!formValid"
                    @click="handleSave"
                >
                    {{
                        editingScheme
                            ? t('permissions.dialog.update')
                            : t('permissions.dialog.create')
                    }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { ref, watch } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import RolePermissionsEditor from './RolePermissionsEditor.vue'

    const { t } = useTranslations()
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
        super_admin: boolean
    }>({
        name: '',
        description: null,
        role_permissions: {},
        super_admin: false,
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
                    const rolePerms = JSON.parse(
                        JSON.stringify(props.editingScheme.role_permissions)
                    )
                    formData.value = {
                        name: props.editingScheme.name,
                        description: props.editingScheme.description ?? null,
                        role_permissions: rolePerms,
                        super_admin: props.editingScheme.super_admin ?? false,
                    }
                    roleNames.value = Object.keys(formData.value.role_permissions)
                } else {
                    formData.value = {
                        name: '',
                        description: null,
                        role_permissions: {},
                        super_admin: false,
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
            constraints: undefined,
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
            super_admin: formData.value.super_admin,
            role_permissions: updatedRolePermissions,
        }

        emit('save', requestData)
    }
</script>
