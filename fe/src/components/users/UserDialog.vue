<template>
    <v-dialog
        :model-value="modelValue"
        :max-width="getDialogMaxWidth('default')"
        persistent
        @update:model-value="$emit('update:modelValue', $event)"
    >
        <v-card>
            <v-card-title class="pa-6">
                {{ editingUser ? t('users.dialog.edit_title') : t('users.dialog.create_title') }}
            </v-card-title>
            <v-card-text class="pa-6">
                <v-form
                    ref="formRef"
                    v-model="formValid"
                >
                    <v-text-field
                        v-if="!editingUser"
                        v-model="formData.username"
                        :label="t('users.dialog.username')"
                        :rules="[rules.required, rules.minLength(3)]"
                        required
                    />
                    <v-text-field
                        v-model="formData.email"
                        :label="t('users.dialog.email')"
                        type="email"
                        :rules="[rules.required, rules.email]"
                        required
                    />
                    <v-text-field
                        v-model="formData.password"
                        :label="t('users.dialog.password')"
                        type="password"
                        :rules="editingUser ? [] : [rules.required, rules.minLength(8)]"
                        :required="!editingUser"
                        :hint="editingUser ? t('users.dialog.password_hint') : ''"
                        persistent-hint
                    />
                    <v-text-field
                        v-model="formData.first_name"
                        :label="t('users.dialog.first_name')"
                        :rules="[rules.required]"
                        required
                    />
                    <v-text-field
                        v-model="formData.last_name"
                        :label="t('users.dialog.last_name')"
                        :rules="[rules.required]"
                        required
                    />
                    <v-select
                        v-model="formData.role_uuids"
                        :label="t('users.dialog.roles')"
                        :items="availableRoles"
                        item-title="name"
                        item-value="uuid"
                        multiple
                        chips
                        :hint="t('users.dialog.roles_hint')"
                        persistent-hint
                        :loading="loadingRoles"
                    />
                    <v-switch
                        v-model="formData.is_active"
                        :label="t('users.dialog.active')"
                        color="primary"
                    />
                    <v-switch
                        v-model="formData.super_admin"
                        :label="t('users.dialog.super_admin')"
                        color="primary"
                    />
                </v-form>
            </v-card-text>
            <v-card-actions class="pa-4 px-6">
                <v-spacer />
                <v-btn
                    variant="text"
                    color="mutedForeground"
                    @click="handleClose"
                >
                    {{ t('users.dialog.cancel') }}
                </v-btn>
                <v-btn
                    color="primary"
                    variant="flat"
                    :loading="loading"
                    :disabled="!formValid"
                    @click="handleSave"
                >
                    {{ editingUser ? t('users.dialog.update') : t('users.dialog.create') }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { ref, watch, onMounted } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useRoles } from '@/composables/useRoles'
    import { getDialogMaxWidth } from '@/design-system/components'
    import type { UserResponse, CreateUserRequest, UpdateUserRequest, Role } from '@/types/schemas'

    const { t } = useTranslations()
    const { loadRoles, roles, loading: loadingRoles } = useRoles()

    interface Props {
        modelValue: boolean
        editingUser: UserResponse | null
        loading: boolean
    }

    const props = defineProps<Props>()

    const emit = defineEmits<{
        'update:modelValue': [value: boolean]
        save: [data: CreateUserRequest | UpdateUserRequest]
    }>()

    const formRef = ref()
    const formValid = ref(false)
    const availableRoles = ref<Role[]>([])

    const formData = ref<CreateUserRequest & { is_active?: boolean; super_admin?: boolean }>({
        username: '',
        email: '',
        password: '',
        first_name: '',
        last_name: '',
        role_uuids: [],
        is_active: true,
        super_admin: false,
    })

    const loadAvailableRoles = async () => {
        try {
            await loadRoles(1, 100) // Load all roles
            availableRoles.value = roles.value
        } catch (err) {
            console.error('Failed to load roles:', err)
        }
    }

    onMounted(async () => {
        await loadAvailableRoles()
    })

    // Watch for dialog opening to reload roles
    watch(
        () => props.modelValue,
        async newValue => {
            if (newValue) {
                // Reload roles when dialog opens
                await loadAvailableRoles()
            } else {
                resetForm()
            }
        }
    )

    const rules = {
        required: (v: string) => !!v || t('users.dialog.validation.required'),
        email: (v: string) => {
            if (!v) {
                return true
            }
            return /.+@.+\..+/.test(v) || t('users.dialog.validation.email_invalid')
        },
        minLength: (min: number) => (v: string) => {
            if (!v) {
                return true
            }
            return v.length >= min || t('users.dialog.validation.min_length', { min: String(min) })
        },
    }

    const resetForm = () => {
        formData.value = {
            username: '',
            email: '',
            password: '',
            first_name: '',
            last_name: '',
            role_uuids: [],
            is_active: true,
            super_admin: false,
        }
        formValid.value = false
    }

    // Watch for editing user changes
    watch(
        () => props.editingUser,
        newUser => {
            if (newUser) {
                formData.value = {
                    username: newUser.username,
                    email: newUser.email,
                    password: '',
                    first_name: newUser.first_name ?? '',
                    last_name: newUser.last_name ?? '',
                    role_uuids: newUser.role_uuids,
                    is_active: newUser.is_active,
                    super_admin: newUser.super_admin,
                }
            } else {
                resetForm()
            }
        },
        { immediate: true }
    )

    const handleClose = () => {
        emit('update:modelValue', false)
        resetForm()
    }

    const handleSave = () => {
        if (!formValid.value) {
            return
        }

        if (props.editingUser) {
            const updateData: UpdateUserRequest = {
                email: formData.value.email,
                first_name: formData.value.first_name,
                last_name: formData.value.last_name,
                role_uuids: formData.value.role_uuids ?? undefined,
                is_active: formData.value.is_active,
                super_admin: formData.value.super_admin,
            }
            // Only include password if provided
            if (formData.value.password) {
                updateData.password = formData.value.password
            }
            emit('save', updateData)
        } else {
            const createData: CreateUserRequest = {
                username: formData.value.username,
                email: formData.value.email,
                password: formData.value.password,
                first_name: formData.value.first_name,
                last_name: formData.value.last_name,
                role_uuids: formData.value.role_uuids ?? undefined,
                is_active: formData.value.is_active,
                super_admin: formData.value.super_admin,
            }
            emit('save', createData)
        }
    }
</script>
