<template>
    <v-dialog
        :model-value="modelValue"
        max-width="600px"
        persistent
        @update:model-value="$emit('update:modelValue', $event)"
    >
        <v-card>
            <v-card-title>
                {{ editingUser ? 'Edit User' : 'Create User' }}
            </v-card-title>
            <v-card-text>
                <v-form
                    ref="formRef"
                    v-model="formValid"
                >
                    <v-text-field
                        v-if="!editingUser"
                        v-model="formData.username"
                        label="Username"
                        :rules="[rules.required, rules.minLength(3)]"
                        required
                    />
                    <v-text-field
                        v-model="formData.email"
                        label="Email"
                        type="email"
                        :rules="[rules.required, rules.email]"
                        required
                    />
                    <v-text-field
                        v-model="formData.password"
                        label="Password"
                        type="password"
                        :rules="editingUser ? [] : [rules.required, rules.minLength(8)]"
                        :required="!editingUser"
                        :hint="editingUser ? 'Leave empty to keep current password' : ''"
                        persistent-hint
                    />
                    <v-text-field
                        v-model="formData.first_name"
                        label="First Name"
                        :rules="[rules.required]"
                        required
                    />
                    <v-text-field
                        v-model="formData.last_name"
                        label="Last Name"
                        :rules="[rules.required]"
                        required
                    />
                    <v-text-field
                        v-model="formData.role"
                        label="Role"
                        hint="Role name (e.g., 'Editor', 'Viewer', 'SuperAdmin')"
                        persistent-hint
                    />
                    <v-checkbox
                        v-model="formData.is_active"
                        label="Active"
                        hint="Whether the user account is active"
                        persistent-hint
                    />
                    <v-checkbox
                        v-model="formData.super_admin"
                        label="Super Admin"
                        hint="Super admin users have all permissions regardless of assigned schemes"
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
                    Cancel
                </v-btn>
                <v-btn
                    color="primary"
                    :loading="loading"
                    :disabled="!formValid"
                    @click="handleSave"
                >
                    {{ editingUser ? 'Update' : 'Create' }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { ref, watch } from 'vue'
    import type { UserResponse, CreateUserRequest, UpdateUserRequest } from '@/types/schemas'

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
        required: (v: string) => !!v || 'This field is required',
        email: (v: string) => {
            if (!v) {
                return true
            }
            return /.+@.+\..+/.test(v) || 'Must be a valid email'
        },
        minLength: (min: number) => (v: string) => {
            if (!v) {
                return true
            }
            return v.length >= min || `Must be at least ${min} characters`
        },
    }

    // Watch for editing user changes
    watch(
        () => props.editingUser,
        newUser => {
            if (newUser) {
                formData.value = {
                    email: newUser.email,
                    password: '',
                    first_name: newUser.first_name || '',
                    last_name: newUser.last_name || '',
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
                role: formData.value.role || undefined,
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
                role: formData.value.role || undefined,
                is_active: formData.value.is_active,
                super_admin: formData.value.super_admin,
            }
            emit('save', createData)
        }
    }
</script>
