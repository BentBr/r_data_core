<template>
    <v-main>
        <v-container
            fluid
            class="fill-height"
        >
            <v-row
                justify="center"
                align="center"
                class="fill-height"
            >
                <v-col
                    cols="12"
                    sm="8"
                    md="6"
                    lg="4"
                    xl="3"
                >
                    <v-card
                        class="elevation-8 pa-8"
                        rounded="lg"
                    >
                        <!-- Header -->
                        <v-card-title class="text-center mb-6">
                            <div class="d-flex justify-space-between align-center mb-2">
                                <div></div>
                                <!-- Spacer -->
                                <LanguageSwitch />
                            </div>
                            <div class="text-h4 font-weight-bold primary--text">R Data Core</div>
                            <div class="text-subtitle-1 grey--text">
                                {{ t('auth.reset_password.title') }}
                            </div>
                        </v-card-title>

                        <v-card-text>
                            <!-- Invalid link error (no token in URL) -->
                            <v-alert
                                v-if="!token"
                                type="error"
                                variant="tonal"
                                class="mb-4"
                                :text="t('auth.reset_password.invalid_link')"
                            />

                            <!-- Success state -->
                            <v-alert
                                v-else-if="succeeded"
                                type="success"
                                variant="tonal"
                                class="mb-4"
                                :text="t('auth.reset_password.success')"
                            />

                            <!-- Reset form -->
                            <v-form
                                v-else
                                ref="resetForm"
                                v-model="formValid"
                                @submit.prevent="handleResetPassword"
                            >
                                <!-- Error alert -->
                                <v-alert
                                    v-if="errorMessage"
                                    type="error"
                                    variant="tonal"
                                    class="mb-4"
                                    :text="errorMessage"
                                    closable
                                    @click:close="errorMessage = ''"
                                />

                                <!-- New Password Field -->
                                <v-text-field
                                    v-model="newPassword"
                                    :type="showNewPassword ? 'text' : 'password'"
                                    :label="t('auth.reset_password.new_password')"
                                    variant="outlined"
                                    :rules="newPasswordRules"
                                    :disabled="isLoading"
                                    class="mb-4"
                                    autofocus
                                >
                                    <template #append-inner>
                                        <SmartIcon
                                            :icon="showNewPassword ? 'eye' : 'eye-off'"
                                            size="sm"
                                            class="cursor-pointer"
                                            @click="showNewPassword = !showNewPassword"
                                        />
                                    </template>
                                </v-text-field>

                                <!-- Confirm Password Field -->
                                <v-text-field
                                    v-model="confirmPassword"
                                    :type="showConfirmPassword ? 'text' : 'password'"
                                    :label="t('auth.reset_password.confirm_password')"
                                    variant="outlined"
                                    :rules="confirmPasswordRules"
                                    :disabled="isLoading"
                                    class="mb-4"
                                >
                                    <template #append-inner>
                                        <SmartIcon
                                            :icon="showConfirmPassword ? 'eye' : 'eye-off'"
                                            size="sm"
                                            class="cursor-pointer"
                                            @click="showConfirmPassword = !showConfirmPassword"
                                        />
                                    </template>
                                </v-text-field>

                                <!-- Submit Button -->
                                <v-btn
                                    type="submit"
                                    block
                                    size="large"
                                    color="primary"
                                    :loading="isLoading"
                                    :disabled="!formValid"
                                >
                                    {{ t('auth.reset_password.submit') }}
                                </v-btn>
                            </v-form>

                            <!-- Back to login link -->
                            <div
                                v-if="!succeeded"
                                class="text-center mt-4"
                            >
                                <v-btn
                                    variant="text"
                                    color="primary"
                                    size="small"
                                    @click="goToLogin"
                                >
                                    {{ t('auth.login.submit') }}
                                </v-btn>
                            </div>
                        </v-card-text>
                    </v-card>
                </v-col>
            </v-row>
        </v-container>
    </v-main>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted } from 'vue'
    import { useRouter, useRoute } from 'vue-router'
    import { useTranslations } from '@/composables/useTranslations'
    import { typedHttpClient } from '@/api/typed-client'
    import LanguageSwitch from '@/components/common/LanguageSwitch.vue'
    import SmartIcon from '@/components/common/SmartIcon.vue'

    const router = useRouter()
    const route = useRoute()
    const { t } = useTranslations()

    // Token from URL
    const token = ref('')

    // Form state
    const resetForm = ref()
    const formValid = ref(false)
    const newPassword = ref('')
    const confirmPassword = ref('')
    const showNewPassword = ref(false)
    const showConfirmPassword = ref(false)
    const isLoading = ref(false)
    const errorMessage = ref('')
    const succeeded = ref(false)

    // Validation rules
    const newPasswordRules = [
        (v: string) => !!v || t('validation.required'),
        (v: string) => v.length >= 8 || t('auth.reset_password.password_too_short'),
    ]

    const confirmPasswordRules = computed(() => [
        (v: string) => !!v || t('validation.required'),
        (v: string) => v === newPassword.value || t('auth.reset_password.passwords_mismatch'),
    ])

    const goToLogin = () => {
        void router.push({ name: 'Login' })
    }

    const handleResetPassword = async () => {
        if (!formValid.value || !token.value) {
            return
        }

        isLoading.value = true
        errorMessage.value = ''
        try {
            await typedHttpClient.resetPassword(token.value, newPassword.value)
            succeeded.value = true
            setTimeout(() => {
                void router.push({ name: 'Login' })
            }, 3000)
        } catch (error) {
            const msg = error instanceof Error ? error.message : t('general.errors.unknown')
            // Token expired / invalid errors from the backend
            errorMessage.value = msg || t('auth.reset_password.invalid_token')
        } finally {
            isLoading.value = false
        }
    }

    onMounted(() => {
        const queryToken = route.query.token
        token.value = (Array.isArray(queryToken) ? queryToken[0] : queryToken) ?? ''
    })
</script>

<style scoped>
    .fill-height {
        min-height: 100vh;
    }

    /* Use theme primary color for background gradient */
    .v-container {
        background: linear-gradient(
            135deg,
            rgb(var(--v-theme-primary)) 0%,
            rgb(var(--v-theme-primary)) 100%
        );
    }

    /* Card uses theme surface color with transparency */
    .v-card {
        background: rgba(var(--v-theme-surface), 0.95);
        backdrop-filter: blur(10px);
    }

    /* Title uses primary color */
    .v-card-title .text-h4 {
        color: rgb(var(--v-theme-primary));
    }
</style>
