<template>
    <div>
        <v-select
            :model-value="authEntityType"
            :items="entityDefItems"
            item-title="title"
            item-value="value"
            :label="t('workflows.dsl.entity_type')"
            density="comfortable"
            class="mb-2"
            :hint="t('workflows.dsl.auth_entity_type_hint')"
            persistent-hint
            @update:model-value="onEntityTypeChange"
        />
        <v-select
            :model-value="authIdentifierField"
            :items="entityFields"
            :label="t('workflows.dsl.auth_identifier_field')"
            density="comfortable"
            class="mb-2"
            :hint="t('workflows.dsl.auth_identifier_field_hint')"
            persistent-hint
            :disabled="entityFields.length === 0"
            @update:model-value="updateField('identifier_field', $event)"
        />
        <v-select
            :model-value="authPasswordField"
            :items="passwordFieldItems"
            :label="t('workflows.dsl.auth_password_field')"
            density="comfortable"
            class="mb-2"
            :hint="passwordFieldHint"
            persistent-hint
            :disabled="entityFields.length === 0"
            @update:model-value="updateField('password_field', $event)"
        />
        <v-text-field
            :model-value="authInputIdentifier"
            :label="t('workflows.dsl.auth_input_identifier')"
            density="comfortable"
            class="mb-2"
            :hint="t('workflows.dsl.auth_input_identifier_hint')"
            persistent-hint
            @update:model-value="updateField('input_identifier', $event)"
        />
        <v-text-field
            :model-value="authInputPassword"
            :label="t('workflows.dsl.auth_input_password')"
            density="comfortable"
            class="mb-2"
            :hint="t('workflows.dsl.auth_input_password_hint')"
            persistent-hint
            @update:model-value="updateField('input_password', $event)"
        />
        <v-text-field
            :model-value="authTargetToken"
            :label="t('workflows.dsl.auth_target_token')"
            density="comfortable"
            class="mb-2"
            :hint="t('workflows.dsl.auth_target_token_hint')"
            persistent-hint
            @update:model-value="updateField('target_token', $event)"
        />
        <v-text-field
            :model-value="authTokenExpiry"
            :label="t('workflows.dsl.auth_token_expiry')"
            type="number"
            density="comfortable"
            class="mb-2"
            :hint="t('workflows.dsl.auth_token_expiry_hint')"
            persistent-hint
            @update:model-value="onExpiryChange"
        />
        <div class="text-caption mb-1">{{ t('workflows.dsl.auth_extra_claims') }}</div>
        <v-btn
            variant="outlined"
            size="small"
            class="mb-2"
            @click="addExtraClaim"
        >
            {{ t('workflows.dsl.add_mapping') }}
        </v-btn>
        <MappingTable
            :pairs="extraClaimPairs"
            :left-label="t('workflows.dsl.auth_claim_name')"
            :right-label="t('workflows.dsl.auth_entity_field')"
            :right-items="entityFields"
            :use-select-for-right="entityFields.length > 0"
            @update-pair="updateExtraClaim"
            @delete-pair="deleteExtraClaim"
        />
    </div>
</template>

<script setup lang="ts">
    import { ref, computed, watch, onMounted } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useEntityDefinitions } from '@/composables/useEntityDefinitions'
    import { typedHttpClient } from '@/api/typed-client'
    import MappingTable from './MappingTable.vue'
    import type { Transform } from './dsl-utils'

    const props = defineProps<{
        modelValue: Transform
        availableFields?: string[]
    }>()

    const emit = defineEmits<{ (e: 'update:modelValue', value: Transform): void }>()

    const { t } = useTranslations()
    const { entityDefinitions, loadEntityDefinitions } = useEntityDefinitions()

    const entityDefItems = ref<{ title: string; value: string }[]>([])
    const entityFields = ref<string[]>([])
    const passwordFields = ref<string[]>([])

    // Computed properties to read current values
    const authEntityType = computed(() => {
        if (props.modelValue.type === 'authenticate') {
            return props.modelValue.entity_type
        }
        return ''
    })
    const authIdentifierField = computed(() => {
        if (props.modelValue.type === 'authenticate') {
            return props.modelValue.identifier_field
        }
        return ''
    })
    const authPasswordField = computed(() => {
        if (props.modelValue.type === 'authenticate') {
            return props.modelValue.password_field
        }
        return ''
    })
    const authInputIdentifier = computed(() => {
        if (props.modelValue.type === 'authenticate') {
            return props.modelValue.input_identifier
        }
        return ''
    })
    const authInputPassword = computed(() => {
        if (props.modelValue.type === 'authenticate') {
            return props.modelValue.input_password
        }
        return ''
    })
    const authTargetToken = computed(() => {
        if (props.modelValue.type === 'authenticate') {
            return props.modelValue.target_token
        }
        return ''
    })
    const authTokenExpiry = computed(() => {
        if (props.modelValue.type === 'authenticate') {
            return props.modelValue.token_expiry_seconds?.toString() ?? ''
        }
        return ''
    })

    // If dedicated Password-type fields exist, show only those; otherwise fall back to all fields
    const passwordFieldItems = computed(() =>
        passwordFields.value.length > 0 ? passwordFields.value : entityFields.value
    )

    const passwordFieldHint = computed(() => {
        if (entityFields.value.length > 0 && passwordFields.value.length === 0) {
            return t('workflows.dsl.auth_password_field_no_password_type')
        }
        return t('workflows.dsl.auth_password_field_hint')
    })

    const extraClaimPairs = computed(() => {
        if (props.modelValue.type === 'authenticate') {
            const claims = props.modelValue.extra_claims ?? {}
            return Object.entries(claims).map(([k, v]) => ({ k, v }))
        }
        return []
    })

    // Populate entity definition dropdown items
    watch(
        () => entityDefinitions.value,
        defs => {
            entityDefItems.value = defs.map(d => ({
                title: d.display_name || d.entity_type,
                value: d.entity_type,
            }))
        },
        { immediate: true }
    )

    onMounted(() => {
        void loadEntityDefinitions()
    })

    // Load fields when entity type changes from model value
    watch(
        () => (props.modelValue.type === 'authenticate' ? props.modelValue.entity_type : undefined),
        entityType => {
            if (entityType) {
                void loadFields(entityType)
            }
        },
        { immediate: true }
    )

    async function loadFields(entityType: string) {
        if (!entityType) {
            entityFields.value = []
            passwordFields.value = []
            return
        }
        try {
            const fields = await typedHttpClient.getEntityFields(entityType)
            const systemFields = [
                'uuid',
                'updated_at',
                'updated_by',
                'created_at',
                'created_by',
                'version',
            ]
            entityFields.value = fields
                .map(f => f.name)
                .filter(name => !systemFields.includes(name))
            passwordFields.value = fields.filter(f => f.type === 'Password').map(f => f.name)
        } catch {
            entityFields.value = []
            passwordFields.value = []
        }
    }

    function onEntityTypeChange(entityType: string) {
        updateField('entity_type', entityType)
        void loadFields(entityType)
    }

    function onExpiryChange(value: string) {
        const num = value ? Number(value) : undefined
        updateField('token_expiry_seconds', num && num > 0 ? num : undefined)
    }

    function updateField(field: string, value: unknown) {
        if (props.modelValue.type === 'authenticate') {
            emit('update:modelValue', { ...props.modelValue, [field]: value })
        }
    }

    function addExtraClaim() {
        if (props.modelValue.type === 'authenticate') {
            const claims = { ...(props.modelValue.extra_claims ?? {}), '': '' }
            emit('update:modelValue', { ...props.modelValue, extra_claims: claims })
        }
    }

    function updateExtraClaim(idx: number, pair: { k: string; v: string }) {
        if (props.modelValue.type === 'authenticate') {
            const pairs = extraClaimPairs.value.map(p => ({ ...p }))
            pairs[idx] = pair
            const claims: Record<string, string> = {}
            for (const { k, v } of pairs) {
                if (k || v) {
                    claims[k] = v
                }
            }
            emit('update:modelValue', {
                ...props.modelValue,
                extra_claims: Object.keys(claims).length > 0 ? claims : undefined,
            })
        }
    }

    function deleteExtraClaim(idx: number) {
        if (props.modelValue.type === 'authenticate') {
            const pairs = extraClaimPairs.value.map(p => ({ ...p }))
            pairs.splice(idx, 1)
            const claims: Record<string, string> = {}
            for (const { k, v } of pairs) {
                if (k || v) {
                    claims[k] = v
                }
            }
            emit('update:modelValue', {
                ...props.modelValue,
                extra_claims: Object.keys(claims).length > 0 ? claims : undefined,
            })
        }
    }
</script>
