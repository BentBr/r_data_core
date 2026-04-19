<template>
    <div>
        <v-alert
            type="info"
            variant="tonal"
            density="compact"
            class="mb-3"
        >
            {{ t('workflows.dsl.hints.send_email.info') }}
        </v-alert>

        <v-select
            :model-value="templateUuid"
            :items="emailTemplateItems"
            item-title="title"
            item-value="value"
            :label="t('workflows.dsl.send_email_template')"
            density="comfortable"
            class="mb-2"
            :hint="t('workflows.dsl.hints.send_email.template_uuid')"
            persistent-hint
            @update:model-value="onTemplateSelected"
        />

        <!-- Template variables hint -->
        <v-alert
            v-if="selectedTemplateVars.length > 0"
            type="info"
            variant="tonal"
            density="compact"
            class="mb-3"
        >
            <div class="text-caption font-weight-bold mb-1">
                {{ t('workflows.dsl.hints.send_email.template_variables') }}
            </div>
            <div
                v-for="v in selectedTemplateVars"
                :key="v.key"
                class="text-caption"
            >
                <code v-text="wrapVar(v.key)" /> — {{ v.description || '(no description)' }}
            </div>
        </v-alert>

        <div class="text-caption mb-1 mt-2">{{ t('workflows.dsl.send_email_to') }}</div>
        <div
            v-for="(operand, idx) in toOperands"
            :key="`to-${idx}`"
            class="d-flex ga-2 mb-2 align-center"
        >
            <v-select
                :model-value="operand.kind"
                :items="stringOperandKinds"
                :label="t('workflows.dsl.value_kind')"
                density="comfortable"
                class="flex-0"
                style="max-width: 180px"
                @update:model-value="updateToKind(idx, $event)"
            />
            <v-select
                v-if="operand.kind === 'field' && availableFields.length > 0"
                :model-value="operand.field"
                :items="availableFields"
                :label="t('workflows.dsl.value')"
                density="comfortable"
                @update:model-value="updateToField(idx, $event)"
            />
            <v-text-field
                v-else-if="operand.kind === 'field'"
                :model-value="operand.field"
                :label="t('workflows.dsl.value')"
                density="comfortable"
                @update:model-value="updateToField(idx, $event)"
            />
            <v-text-field
                v-else
                :model-value="operand.value"
                :label="t('workflows.dsl.value')"
                density="comfortable"
                @update:model-value="updateToValue(idx, $event)"
            />
            <v-btn
                icon="mdi-delete"
                variant="text"
                size="small"
                @click="removeTo(idx)"
            />
        </div>
        <v-btn
            variant="outlined"
            size="small"
            class="mb-4"
            :hint="t('workflows.dsl.hints.send_email.to')"
            persistent-hint
            @click="addTo"
        >
            {{ t('workflows.dsl.add_recipient') }}
        </v-btn>
        <div class="text-caption text-medium-emphasis mb-2">
            {{ t('workflows.dsl.hints.send_email.to') }}
        </div>

        <div class="text-caption mb-1 mt-2">{{ t('workflows.dsl.send_email_cc') }}</div>
        <div
            v-for="(operand, idx) in ccOperands"
            :key="`cc-${idx}`"
            class="d-flex ga-2 mb-2 align-center"
        >
            <v-select
                :model-value="operand.kind"
                :items="stringOperandKinds"
                :label="t('workflows.dsl.value_kind')"
                density="comfortable"
                class="flex-0"
                style="max-width: 180px"
                @update:model-value="updateCcKind(idx, $event)"
            />
            <v-select
                v-if="operand.kind === 'field' && availableFields.length > 0"
                :model-value="operand.field"
                :items="availableFields"
                :label="t('workflows.dsl.value')"
                density="comfortable"
                @update:model-value="updateCcField(idx, $event)"
            />
            <v-text-field
                v-else-if="operand.kind === 'field'"
                :model-value="operand.field"
                :label="t('workflows.dsl.value')"
                density="comfortable"
                @update:model-value="updateCcField(idx, $event)"
            />
            <v-text-field
                v-else
                :model-value="operand.value"
                :label="t('workflows.dsl.value')"
                density="comfortable"
                @update:model-value="updateCcValue(idx, $event)"
            />
            <v-btn
                icon="mdi-delete"
                variant="text"
                size="small"
                @click="removeCc(idx)"
            />
        </div>
        <v-btn
            variant="outlined"
            size="small"
            class="mb-2"
            @click="addCc"
        >
            {{ t('workflows.dsl.add_cc_recipient') }}
        </v-btn>
        <div class="text-caption text-medium-emphasis mb-4">
            {{ t('workflows.dsl.hints.send_email.cc') }}
        </div>

        <v-text-field
            :model-value="targetStatus"
            :label="t('workflows.dsl.send_email_target_status')"
            density="comfortable"
            class="mb-2"
            :hint="t('workflows.dsl.hints.send_email.target_status')"
            persistent-hint
            @update:model-value="updateField('target_status', $event)"
        />
    </div>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { typedHttpClient } from '@/api/typed-client'
    import type { Transform, StringOperand } from './dsl-utils'

    const props = defineProps<{
        modelValue: Transform
        availableFields?: string[]
    }>()

    const emit = defineEmits<{ (e: 'update:modelValue', value: Transform): void }>()

    const { t } = useTranslations()

    const availableFields = computed(() => props.availableFields ?? [])
    const stringOperandKinds = ['field', 'const_string']

    import type { EmailTemplate } from '@/api/clients/email-templates'

    const emailTemplates = ref<EmailTemplate[]>([])
    const emailTemplateItems = ref<{ title: string; value: string }[]>([])
    const selectedTemplateVars = ref<Array<{ key: string; description: string }>>([])

    const wrapVar = (key: string): string => `{{${key}}}`

    const templateUuid = computed(() => {
        if (props.modelValue.type === 'send_email') {
            return props.modelValue.template_uuid
        }
        return ''
    })

    const toOperands = computed((): StringOperand[] => {
        if (props.modelValue.type === 'send_email') {
            return props.modelValue.to
        }
        return []
    })

    const ccOperands = computed((): StringOperand[] => {
        if (props.modelValue.type === 'send_email') {
            return props.modelValue.cc ?? []
        }
        return []
    })

    const targetStatus = computed(() => {
        if (props.modelValue.type === 'send_email') {
            return props.modelValue.target_status
        }
        return ''
    })

    onMounted(() => {
        void loadEmailTemplates()
    })

    async function loadEmailTemplates() {
        try {
            const templates = await typedHttpClient.listEmailTemplates('workflow')
            emailTemplates.value = templates
            emailTemplateItems.value = templates.map(tmpl => ({
                title: tmpl.name,
                value: tmpl.uuid,
            }))
            // If a template is already selected, load its variables
            if (templateUuid.value) {
                updateSelectedVars(templateUuid.value)
            }
        } catch {
            emailTemplates.value = []
            emailTemplateItems.value = []
        }
    }

    function updateSelectedVars(uuid: string) {
        const tmpl = emailTemplates.value.find(t => t.uuid === uuid)
        if (tmpl) {
            const vars = tmpl.variables as Array<{ key: string; description: string }>
            selectedTemplateVars.value = Array.isArray(vars) ? vars : []
        } else {
            selectedTemplateVars.value = []
        }
    }

    function onTemplateSelected(uuid: string) {
        updateField('template_uuid', uuid)
        updateSelectedVars(uuid)
    }

    function updateField(field: string, value: unknown) {
        if (props.modelValue.type === 'send_email') {
            emit('update:modelValue', { ...props.modelValue, [field]: value })
        }
    }

    // To operand helpers
    function addTo() {
        if (props.modelValue.type === 'send_email') {
            const newOperand: StringOperand = { kind: 'const_string', value: '' }
            emit('update:modelValue', {
                ...props.modelValue,
                to: [...props.modelValue.to, newOperand],
            })
        }
    }

    function removeTo(idx: number) {
        if (props.modelValue.type === 'send_email') {
            const updated = props.modelValue.to.filter((_, i) => i !== idx)
            emit('update:modelValue', { ...props.modelValue, to: updated })
        }
    }

    function updateToKind(idx: number, kind: 'field' | 'const_string') {
        if (props.modelValue.type === 'send_email') {
            const updated = props.modelValue.to.map((op, i) => {
                if (i !== idx) return op
                return kind === 'field'
                    ? ({ kind: 'field', field: '' } as StringOperand)
                    : ({ kind: 'const_string', value: '' } as StringOperand)
            })
            emit('update:modelValue', { ...props.modelValue, to: updated })
        }
    }

    function updateToField(idx: number, fieldValue: string) {
        if (props.modelValue.type === 'send_email') {
            const updated = props.modelValue.to.map((op, i) => {
                if (i !== idx) return op
                return { kind: 'field' as const, field: fieldValue }
            })
            emit('update:modelValue', { ...props.modelValue, to: updated })
        }
    }

    function updateToValue(idx: number, val: string) {
        if (props.modelValue.type === 'send_email') {
            const updated = props.modelValue.to.map((op, i) => {
                if (i !== idx) return op
                return { kind: 'const_string' as const, value: val }
            })
            emit('update:modelValue', { ...props.modelValue, to: updated })
        }
    }

    // CC operand helpers
    function addCc() {
        if (props.modelValue.type === 'send_email') {
            const newOperand: StringOperand = { kind: 'const_string', value: '' }
            const existing = props.modelValue.cc ?? []
            emit('update:modelValue', {
                ...props.modelValue,
                cc: [...existing, newOperand],
            })
        }
    }

    function removeCc(idx: number) {
        if (props.modelValue.type === 'send_email') {
            const existing = props.modelValue.cc ?? []
            const updated = existing.filter((_, i) => i !== idx)
            emit('update:modelValue', {
                ...props.modelValue,
                cc: updated.length > 0 ? updated : undefined,
            })
        }
    }

    function updateCcKind(idx: number, kind: 'field' | 'const_string') {
        if (props.modelValue.type === 'send_email') {
            const existing = props.modelValue.cc ?? []
            const updated = existing.map((op, i) => {
                if (i !== idx) return op
                return kind === 'field'
                    ? ({ kind: 'field', field: '' } as StringOperand)
                    : ({ kind: 'const_string', value: '' } as StringOperand)
            })
            emit('update:modelValue', { ...props.modelValue, cc: updated })
        }
    }

    function updateCcField(idx: number, fieldValue: string) {
        if (props.modelValue.type === 'send_email') {
            const existing = props.modelValue.cc ?? []
            const updated = existing.map((op, i) => {
                if (i !== idx) return op
                return { kind: 'field' as const, field: fieldValue }
            })
            emit('update:modelValue', { ...props.modelValue, cc: updated })
        }
    }

    function updateCcValue(idx: number, val: string) {
        if (props.modelValue.type === 'send_email') {
            const existing = props.modelValue.cc ?? []
            const updated = existing.map((op, i) => {
                if (i !== idx) return op
                return { kind: 'const_string' as const, value: val }
            })
            emit('update:modelValue', { ...props.modelValue, cc: updated })
        }
    }
</script>

<style scoped>
    .ga-2 {
        gap: 8px;
    }
</style>
