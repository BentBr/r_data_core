<template>
    <v-container fluid>
        <v-row>
            <v-col cols="12">
                <v-card>
                    <v-card-title class="text-h4 pa-4">
                        <v-icon
                            icon="mdi-cog"
                            class="mr-3"
                        />
                        {{ t('system.admin.title') }}
                    </v-card-title>
                    <v-card-text>
                        <v-row>
                            <v-col cols="12" md="6">
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
                                            :loading="saving"
                                            @click="save"
                                        >
                                            {{ t('system.versioning.save') }}
                                        </v-btn>
                                    </v-card-actions>
                                </v-card>
                            </v-col>
                        </v-row>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>
    </v-container>
</template>

<script setup lang="ts">
    import { onMounted, ref } from 'vue'
    import { useAuthStore } from '@/stores/auth'
    import { useSnackbar } from '@/composables/useSnackbar'
    import { useTranslations } from '@/composables/useTranslations'
    import { typedHttpClient } from '@/api/typed-client'

    const { showSuccess, showError } = useSnackbar()
    const { t } = useTranslations()
    const auth = useAuthStore()

    const form = ref<{ enabled: boolean; max_versions: number | null; max_age_days: number | null }>(
        {
            enabled: true,
            max_versions: null,
            max_age_days: 180,
        }
    )
    const loading = ref(false)
    const saving = ref(false)

    const load = async () => {
        loading.value = true
        try {
            const settings = await typedHttpClient.getEntityVersioningSettings()
            form.value = settings || form.value
        } catch (e: any) {
            showError(t('system.versioning.load_failed'))
        } finally {
            loading.value = false
        }
    }

    const save = async () => {
        saving.value = true
        try {
            await typedHttpClient.updateEntityVersioningSettings(form.value)
            showSuccess(t('system.versioning.save_success'))
        } catch (e: any) {
            showError(t('system.versioning.save_failed'))
        } finally {
            saving.value = false
        }
    }

    onMounted(() => {
        void load()
    })
</script>
