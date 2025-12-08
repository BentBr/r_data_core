<template>
    <div>
        <PageLayout>
            <v-row>
                <v-col
                    cols="12"
                    md="6"
                >
                    <v-card variant="outlined">
                        <v-card-title class="text-subtitle-1 pa-3">
                            {{ t('system.versioning.section_title') }}
                        </v-card-title>
                        <v-card-text class="pa-3">
                            <v-switch
                                v-model="form.enabled"
                                :label="t('system.versioning.enabled')"
                                color="success"
                                inset
                            />
                            <v-text-field
                                v-model="form.max_versions"
                                :label="t('system.versioning.max_versions')"
                                type="number"
                                min="0"
                            />
                            <v-text-field
                                v-model="form.max_age_days"
                                :label="t('system.versioning.max_age_days')"
                                type="number"
                                min="0"
                            />
                        </v-card-text>
                        <v-card-actions>
                            <v-spacer />
                            <v-btn
                                color="primary"
                                variant="flat"
                                :loading="saving"
                                @click="save"
                            >
                                {{ t('system.versioning.save') }}
                            </v-btn>
                        </v-card-actions>
                    </v-card>
                </v-col>
            </v-row>

            <SnackbarManager :snackbar="currentSnackbar" />
        </PageLayout>
    </div>
</template>

<script setup lang="ts">
    import { onMounted, ref } from 'vue'
    import { useSnackbar } from '@/composables/useSnackbar'
    import { useTranslations } from '@/composables/useTranslations'
    import { typedHttpClient } from '@/api/typed-client'
    import PageLayout from '@/components/layouts/PageLayout.vue'
    import SnackbarManager from '@/components/common/SnackbarManager.vue'

    const { currentSnackbar, showSuccess, showError } = useSnackbar()
    const { t } = useTranslations()

    const form = ref<{
        enabled: boolean
        max_versions: number | string | null
        max_age_days: number | string | null
    }>({
        enabled: true,
        max_versions: null,
        max_age_days: 180,
    })
    const loading = ref(false)
    const saving = ref(false)

    const load = async () => {
        loading.value = true
        try {
            const settings = await typedHttpClient.getEntityVersioningSettings()
            if (settings) {
                form.value = {
                    enabled: settings.enabled,
                    max_versions: settings.max_versions ?? null,
                    max_age_days: settings.max_age_days ?? null,
                }
            }
        } catch {
            showError(t('system.versioning.load_failed'))
        } finally {
            loading.value = false
        }
    }

    const save = async () => {
        saving.value = true
        try {
            // Convert string inputs to numbers or null
            const payload = {
                enabled: form.value.enabled,
                max_versions:
                    form.value.max_versions === null ||
                    form.value.max_versions === '' ||
                    Number.isNaN(Number(form.value.max_versions))
                        ? null
                        : Number(form.value.max_versions),
                max_age_days:
                    form.value.max_age_days === null ||
                    form.value.max_age_days === '' ||
                    Number.isNaN(Number(form.value.max_age_days))
                        ? null
                        : Number(form.value.max_age_days),
            }
            await typedHttpClient.updateEntityVersioningSettings(payload)
            showSuccess(t('system.versioning.save_success'))
        } catch {
            showError(t('system.versioning.save_failed'))
        } finally {
            saving.value = false
        }
    }

    onMounted(() => {
        void load()
    })
</script>
