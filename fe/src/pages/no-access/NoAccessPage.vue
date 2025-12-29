<template>
    <PageLayout
        title="No Access"
        icon="shield-off"
    >
        <v-row>
            <v-col
                cols="12"
                class="text-center"
            >
                <div class="mb-6">
                    <SmartIcon
                        icon="shield-off"
                        :size="80"
                        color="warning"
                    />
                </div>
                <h2 class="text-h4 mb-4">
                    {{ t('no_access.title') }}
                </h2>
                <p class="text-body-1 mb-6">
                    {{ t('no_access.message') }}
                </p>
                <v-card
                    v-if="authStore.user"
                    variant="outlined"
                    class="mx-auto"
                    max-width="400"
                >
                    <v-card-text>
                        <div class="text-subtitle-1 mb-2">
                            {{ t('no_access.logged_in_as') }}
                        </div>
                        <div class="text-h6">
                            {{ authStore.user.username }}
                        </div>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>
    </PageLayout>
</template>

<script setup lang="ts">
    import { useAuthStore } from '@/stores/auth'
    import { useTranslations } from '@/composables/useTranslations'
    import PageLayout from '@/components/layouts/PageLayout.vue'
    import SmartIcon from '@/components/common/SmartIcon.vue'

    const authStore = useAuthStore()
    const { t } = useTranslations()
</script>

<style scoped>
    /* Component-specific styles */
</style>
