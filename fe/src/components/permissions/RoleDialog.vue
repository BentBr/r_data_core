<template>
    <v-dialog
        :model-value="modelValue"
        :max-width="getDialogMaxWidth('form')"
        persistent
        @update:model-value="$emit('update:modelValue', $event)"
    >
        <v-card>
            <v-card-title class="pa-6">
                {{ editingRole ? t('roles.dialog.edit_title') : t('roles.dialog.create_title') }}
            </v-card-title>
            <v-card-text class="pa-6">
                <v-form
                    ref="formRef"
                    v-model="formValid"
                >
                    <v-text-field
                        v-model="formData.name"
                        :label="t('roles.dialog.name')"
                        :rules="[rules.required]"
                        required
                    />
                    <v-textarea
                        v-model="formData.description"
                        :label="t('roles.dialog.description')"
                        rows="2"
                    />

                    <v-switch
                        v-model="formData.super_admin"
                        :label="t('roles.dialog.super_admin')"
                        color="primary"
                    />

                    <v-divider class="my-4" />

                    <div class="text-h6 mb-4">{{ t('roles.dialog.permissions') }}</div>

                    <v-alert
                        v-if="formData.super_admin"
                        type="info"
                        variant="tonal"
                        class="mb-4"
                    >
                        {{ t('roles.dialog.super_admin_disabled_hint') }}
                    </v-alert>

                    <PermissionEditor
                        v-for="(permission, index) in formData.permissions"
                        :key="index"
                        :permission="permission"
                        :resource-types="resourceTypes"
                        :permission-types="permissionTypes"
                        :access-levels="accessLevels"
                        :disabled="formData.super_admin"
                        class="mb-4"
                        @update="updatePermission(index, $event)"
                        @remove="removePermission(index)"
                    />

                    <v-btn
                        color="primary"
                        variant="outlined"
                        :disabled="formData.super_admin"
                        @click="addPermission"
                    >
                        <template #prepend>
                            <SmartIcon
                                icon="plus"
                                size="sm"
                            />
                        </template>
                        {{ t('roles.dialog.add_permission') }}
                    </v-btn>
                </v-form>
            </v-card-text>
            <v-card-actions class="pa-4 px-6">
                <v-spacer />
                <v-btn
                    variant="text"
                    color="mutedForeground"
                    @click="handleClose"
                >
                    {{ t('roles.dialog.cancel') }}
                </v-btn>
                <v-btn
                    color="primary"
                    variant="flat"
                    :loading="loading"
                    :disabled="!formValid"
                    @click="handleSave"
                >
                    {{ editingRole ? t('roles.dialog.update') : t('roles.dialog.create') }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import { ref, watch } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { getDialogMaxWidth } from '@/design-system/components'
    import PermissionEditor from './PermissionEditor.vue'

    const { t } = useTranslations()
    import type {
        Role,
        CreateRoleRequest,
        UpdateRoleRequest,
        Permission,
        ResourceNamespace,
        PermissionType,
        AccessLevel,
    } from '@/types/schemas'

    interface Props {
        modelValue: boolean
        editingRole?: Role | null
        loading?: boolean
        resourceTypes: ResourceNamespace[]
        permissionTypes: PermissionType[]
        accessLevels: AccessLevel[]
    }

    interface Emits {
        (e: 'update:modelValue', value: boolean): void

        (e: 'save', data: CreateRoleRequest | UpdateRoleRequest): void
    }

    const props = withDefaults(defineProps<Props>(), {
        editingRole: null,
        loading: false,
    })

    const emit = defineEmits<Emits>()

    const formRef = ref()
    const formValid = ref(false)

    const formData = ref<{
        name: string
        description: string | null
        permissions: Permission[]
        super_admin: boolean
    }>({
        name: '',
        description: null,
        permissions: [],
        super_admin: false,
    })

    const rules = {
        required: (v: string) => !!v || 'Required',
    }

    // Reset form when dialog opens/closes or editingRole changes
    watch(
        () => [props.modelValue, props.editingRole],
        () => {
            if (props.modelValue) {
                if (props.editingRole) {
                    formData.value = {
                        name: props.editingRole.name,
                        description: props.editingRole.description ?? null,
                        permissions: JSON.parse(JSON.stringify(props.editingRole.permissions)),
                        super_admin: props.editingRole.super_admin,
                    }
                } else {
                    formData.value = {
                        name: '',
                        description: null,
                        permissions: [],
                        super_admin: false,
                    }
                }
            }
        },
        { immediate: true }
    )

    const addPermission = () => {
        formData.value.permissions.push({
            resource_type: 'Workflows',
            permission_type: 'Read',
            access_level: 'All',
            resource_uuids: [],
            constraints: undefined,
        })
    }

    const removePermission = (index: number) => {
        formData.value.permissions.splice(index, 1)
    }

    const updatePermission = (index: number, permission: Permission) => {
        formData.value.permissions[index] = permission
    }

    const handleClose = () => {
        emit('update:modelValue', false)
    }

    const handleSave = () => {
        if (!formValid.value) {
            return
        }

        const requestData = {
            name: formData.value.name,
            description: formData.value.description,
            super_admin: formData.value.super_admin,
            permissions: formData.value.permissions,
        }

        emit('save', requestData)
    }
</script>
