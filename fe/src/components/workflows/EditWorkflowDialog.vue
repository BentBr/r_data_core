<template>
  <v-dialog v-model="model" max-width="720px">
    <v-card>
      <v-card-title>Edit Workflow</v-card-title>
      <v-card-text>
        <v-form ref="formRef" @submit.prevent>
          <v-text-field v-model="form.name" label="Name" :rules="[rules.required]" />
          <v-textarea v-model="form.description" label="Description" rows="2" auto-grow />
          <v-select
            v-model="form.kind"
            label="Kind"
            :items="kinds"
            item-title="label"
            item-value="value"
          />
          <v-switch v-model="form.enabled" label="Enabled" inset></v-switch>
          <v-text-field v-model="form.schedule_cron" label="Cron" :error-messages="cronError || ''" @update:model-value="onCronChange" />
          <div class="text-caption mb-2" v-if="cronHelp">
            {{ cronHelp }}
          </div>
          <div class="text-caption" v-if="nextRuns.length">
            Next: {{ nextRuns.join(', ') }}
          </div>

          <v-expansion-panels class="mt-2">
            <v-expansion-panel>
              <v-expansion-panel-title>Config (JSON)</v-expansion-panel-title>
              <v-expansion-panel-text>
                <v-textarea v-model="configJson" rows="8" auto-grow :error-messages="configError || ''" />
              </v-expansion-panel-text>
            </v-expansion-panel>
          </v-expansion-panels>
        </v-form>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="cancel">Cancel</v-btn>
        <v-btn color="primary" :loading="loading" @click="submit">Save</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { ValidationError } from '@/api/typed-client'

const props = defineProps<{ modelValue: boolean; workflowUuid: string | null }>()
const emit = defineEmits<{ (e: 'update:modelValue', value: boolean): void; (e: 'updated'): void }>()

const model = computed({ get: () => props.modelValue, set: v => emit('update:modelValue', v) })
const loading = ref(false)
const formRef = ref()

const kinds = [
  { label: 'Consumer', value: 'consumer' },
  { label: 'Provider', value: 'provider' },
]

const form = ref({
  name: '',
  description: '',
  kind: 'consumer' as 'consumer' | 'provider',
  enabled: true,
  schedule_cron: '' as string | null,
})

const configJson = ref('')
const configError = ref<string | null>(null)
const cronError = ref<string | null>(null)
const cronHelp = ref<string>('Use standard 5-field cron (min hour day month dow), e.g. "*/5 * * * *"')
const nextRuns = ref<string[]>([])
let cronDebounce: any = null

const rules = {
  required: (v: any) => !!v || 'Required',
}

watch(() => props.modelValue, (open) => {
  if (open) void loadDetails()
})

async function loadDetails() {
  if (!props.workflowUuid) return
  loading.value = true
  try {
    const data = await typedHttpClient.getWorkflow(props.workflowUuid)
    form.value.name = data.name
    form.value.description = data.description ?? ''
    form.value.kind = data.kind
    form.value.enabled = data.enabled
    form.value.schedule_cron = data.schedule_cron ?? ''
    configJson.value = JSON.stringify(data.config ?? {}, null, 2)
  } finally {
    loading.value = false
  }
}

async function onCronChange(value: string) {
  cronError.value = null
  if (cronDebounce) clearTimeout(cronDebounce)
  if (!value || !value.trim()) { nextRuns.value = []; return }
  cronDebounce = setTimeout(async () => {
    try {
      nextRuns.value = await typedHttpClient.previewCron(value)
    } catch (e) {
      nextRuns.value = []
    }
  }, 350)
}

function cancel() { model.value = false }

function parseJson(input: string): any | undefined {
  if (!input || !input.trim()) return undefined
  try { return JSON.parse(input) } catch { return null }
}

async function submit() {
  if (!props.workflowUuid) return
  configError.value = null
  cronError.value = null
  const parsedConfig = parseJson(configJson.value)
  if (parsedConfig === null) { configError.value = 'Invalid JSON'; return }

  loading.value = true
  try {
    await typedHttpClient.updateWorkflow(props.workflowUuid, {
      name: form.value.name,
      description: form.value.description || null,
      kind: form.value.kind,
      enabled: form.value.enabled,
      schedule_cron: form.value.schedule_cron || null,
      config: parsedConfig ?? {},
    })
    emit('updated')
    model.value = false
  } catch (e: any) {
    if (e instanceof ValidationError) {
      const cronViolation = e.violations.find(v => v.field === 'schedule_cron')
      if (cronViolation) cronError.value = cronViolation.message
      return
    }
    throw e
  } finally {
    loading.value = false
  }
}
</script>


