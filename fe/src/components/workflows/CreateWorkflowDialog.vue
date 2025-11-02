<template>
  <v-dialog v-model="model" max-width="720px">
    <v-card>
      <v-card-title>{{ t('workflows.create.title') }}</v-card-title>
      <v-card-text>
        <v-form ref="formRef" @submit.prevent>
          <v-text-field v-model="form.name" :label="t('workflows.create.name')" :rules="[rules.required]" />
          <v-textarea v-model="form.description" :label="t('workflows.create.description')" rows="2" auto-grow />
          <v-select
            v-model="form.kind"
            :label="t('workflows.create.kind')"
            :items="kinds"
            item-title="label"
            item-value="value"
          />
          <v-switch v-model="form.enabled" :label="t('workflows.create.enabled')" inset></v-switch>
          <v-text-field v-model="form.schedule_cron" :label="t('workflows.create.cron')" />

          <v-expansion-panels class="mt-2">
            <v-expansion-panel>
              <v-expansion-panel-title>{{ t('workflows.create.consumer_config') }}</v-expansion-panel-title>
              <v-expansion-panel-text>
                <v-textarea v-model="consumerJson" rows="6" auto-grow :error-messages="consumerError || ''" />
              </v-expansion-panel-text>
            </v-expansion-panel>
            <v-expansion-panel>
              <v-expansion-panel-title>{{ t('workflows.create.provider_config') }}</v-expansion-panel-title>
              <v-expansion-panel-text>
                <v-textarea v-model="providerJson" rows="6" auto-grow :error-messages="providerError || ''" />
              </v-expansion-panel-text>
            </v-expansion-panel>
          </v-expansion-panels>
        </v-form>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="cancel">{{ t('common.cancel') }}</v-btn>
        <v-btn color="primary" :loading="loading" @click="submit">{{ t('workflows.create.create_button') }}</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
  
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { useTranslations } from '@/composables/useTranslations'

const props = defineProps<{ modelValue: boolean }>()
const emit = defineEmits<{ (e: 'update:modelValue', value: boolean): void; (e: 'created', uuid: string): void }>()

const { t } = useTranslations()
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

const consumerJson = ref('')
const providerJson = ref('')
const consumerError = ref<string | null>(null)
const providerError = ref<string | null>(null)

const rules = {
  required: (v: any) => (!!v && String(v).trim().length > 0) || t('validation.required'),
}

function parseJson(input: string): any | undefined {
  if (!input || !input.trim()) return undefined
  try {
    return JSON.parse(input)
  } catch (e) {
    return null
  }
}

function cancel() { model.value = false }

async function submit() {
  consumerError.value = null
  providerError.value = null
  const consumer = parseJson(consumerJson.value)
  const provider = parseJson(providerJson.value)
  if (consumer === null) { consumerError.value = t('workflows.create.json_invalid'); return }
  if (provider === null) { providerError.value = t('workflows.create.json_invalid'); return }

  loading.value = true
  try {
    const payload = {
      name: form.value.name,
      description: form.value.description || null,
      kind: form.value.kind,
      enabled: form.value.enabled,
      schedule_cron: form.value.schedule_cron || null,
      consumer_config: consumer,
      provider_config: provider,
    }
    const res = await typedHttpClient.createWorkflow(payload)
    emit('created', res.uuid)
    model.value = false
  } finally {
    loading.value = false
  }
}
</script>


