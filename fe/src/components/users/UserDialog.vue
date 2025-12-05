<template>
    <v-dialog
        :model-value="modelValue"
        max-width="600px"
        persistent
        @update:model-value="$emit('update:modelValue', $event)"
    >
        <v-card>
            <v-card-title>
                {{ editingUser ? t('users.dialog.edit_title') : t('users.dialog.create_title') }}
            </v-card-title>
            <v-card-text>
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
                    <v-text-field
                        v-model="formData.role"
                        :label="t('users.dialog.role')"
                        :hint="t('users.dialog.role_hint')"
                        persistent-hint
                    />
                    <v-checkbox
                        v-model="formData.is_active"
                        :label="t('users.dialog.active')"
                        :hint="t('users.dialog.active_hint')"
                        persistent-hint
                    />
                    <v-checkbox
                        v-model="formData.super_admin"
                        :label="t('users.dialog.super_admin')"
                        :hint="t('users.dialog.super_admin_hint')"
                        persistent-hint
                    />
                </v-form>
            </v-card-text>
            <v-card-actions>
                <v-spacer />
                <v-btn
                    variant="text"
                    @click="handleClose"
                >
                    {{ t('users.dialog.cancel') }}
                </v-btn>
                <v-btn
                    color="primary"
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
    import { ref, watch } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import type { UserResponse, CreateUserRequest, UpdateUserRequest } from '@/types/schemas'

    const { t } = useTranslations()

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

    const formData = ref<CreateUserRequest & { is_active?: boolean; super_admin?: boolean }>({
        username: '',
        email: '',
        password: '',
        first_name: '',
        last_name: '',
        role: '',
        is_active: true,
        super_admin: false,
    })

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
            return v.length >= min || t('users.dialog.validation.min_length', { min })
        },
    }

    const resetForm = () => {
        formData.value = {
            username: '',
            email: '',
            password: '',
            first_name: '',
            last_name: '',
            role: '',
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
                    email: newUser.email,
                    password: '',
                    first_name: newUser.first_name ?? '',
                    last_name: newUser.last_name ?? '',
                    role: newUser.role,
                    is_active: newUser.is_active,
                    super_admin: newUser.super_admin,
                }
            } else {
                resetForm()
            }
        },
        { immediate: true }
    )

    // Watch for dialog state changes
    watch(
        () => props.modelValue,
        newValue => {
            if (!newValue) {
                resetForm()
            }
        }
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
                role: formData.value.role ?? undefined,
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
                role: formData.value.role ?? undefined,
                is_active: formData.value.is_active,
                super_admin: formData.value.super_admin,
            }
            emit('save', createData)
        }
    }
</script>
