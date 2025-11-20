<template>
    <div>
        <v-select
            :model-value="authType"
            :items="authTypes"
            :label="t('workflows.dsl.auth_type')"
            density="comfortable"
            class="mb-2"
            @update:model-value="onAuthTypeChange"
        />
        <template v-if="authType === 'api_key'">
            <v-text-field
                :model-value="(modelValue as any).key"
                :label="t('workflows.dsl.api_key')"
                type="password"
                density="comfortable"
                class="mb-2"
                @update:model-value="updateField('key', $event)"
            />
            <v-text-field
                :model-value="(modelValue as any).header_name || 'X-API-Key'"
                :label="t('workflows.dsl.header_name')"
                density="comfortable"
                @update:model-value="updateField('header_name', $event)"
            />
        </template>
        <template v-else-if="authType === 'basic_auth'">
            <v-text-field
                :model-value="(modelValue as any).username"
                :label="t('workflows.dsl.username')"
                density="comfortable"
                class="mb-2"
                @update:model-value="updateField('username', $event)"
            />
            <v-text-field
                :model-value="(modelValue as any).password"
                :label="t('workflows.dsl.password')"
                type="password"
                density="comfortable"
                @update:model-value="updateField('password', $event)"
            />
        </template>
        <template v-else-if="authType === 'pre_shared_key'">
            <v-text-field
                :model-value="(modelValue as any).key"
                :label="t('workflows.dsl.pre_shared_key')"
                type="password"
                density="comfortable"
                class="mb-2"
                @update:model-value="updateField('key', $event)"
            />
            <v-select
                :model-value="(modelValue as any).location || 'header'"
                :items="keyLocations"
                :label="t('workflows.dsl.key_location')"
                density="comfortable"
                class="mb-2"
                @update:model-value="updateField('location', $event)"
            />
            <v-text-field
                :model-value="(modelValue as any).field_name"
                :label="t('workflows.dsl.field_name')"
                density="comfortable"
                @update:model-value="updateField('field_name', $event)"
            />
        </template>
    </div>
</template>

<script setup lang="ts">
    import { computed } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import type { AuthConfig } from './dsl-utils'

    const props = defineProps<{
        modelValue: AuthConfig
    }>()

    const emit = defineEmits<{
        'update:modelValue': [value: AuthConfig]
    }>()

    const { t } = useTranslations()

    const authType = computed(() => props.modelValue?.type || 'none')

    const authTypes = [
        { title: t('workflows.dsl.auth_none'), value: 'none' },
        { title: t('workflows.dsl.auth_api_key'), value: 'api_key' },
        { title: t('workflows.dsl.auth_basic'), value: 'basic_auth' },
        { title: t('workflows.dsl.auth_pre_shared_key'), value: 'pre_shared_key' },
    ]

    const keyLocations = [
        { title: t('workflows.dsl.key_location_header'), value: 'header' },
        { title: t('workflows.dsl.key_location_body'), value: 'body' },
    ]

    function onAuthTypeChange(newType: string) {
        let newAuth: AuthConfig
        switch (newType) {
            case 'none':
                newAuth = { type: 'none' }
                break
            case 'api_key':
                newAuth = {
                    type: 'api_key',
                    key: '',
                    header_name: 'X-API-Key',
                }
                break
            case 'basic_auth':
                newAuth = {
                    type: 'basic_auth',
                    username: '',
                    password: '',
                }
                break
            case 'pre_shared_key':
                newAuth = {
                    type: 'pre_shared_key',
                    key: '',
                    location: 'header',
                    field_name: '',
                }
                break
            default:
                newAuth = { type: 'none' }
        }
        emit('update:modelValue', newAuth)
    }

    function updateField(field: string, value: unknown) {
        const updated = { ...props.modelValue, [field]: value }
        emit('update:modelValue', updated as AuthConfig)
    }
</script>
