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
                            <div class="text-subtitle-1 grey--text">Admin Interface</div>
                        </v-card-title>

                        <!-- Login Form -->
                        <v-card-text>
                            <v-form
                                ref="loginForm"
                                v-model="formValid"
                                @submit.prevent="handleLogin"
                            >
                                <!-- Username Field -->
                                <v-text-field
                                    v-model="credentials.username"
                                    :label="t('auth.login.username')"
                                    variant="outlined"
                                    :rules="usernameRules"
                                    :error-messages="fieldErrors.username"
                                    :disabled="authStore.isLoading"
                                    class="mb-4"
                                    autofocus
                                    @input="clearFieldError('username')"
                                >
                                    <template #prepend-inner>
                                        <SmartIcon
                                            icon="user"
                                            size="sm"
                                            class="mr-2"
                                        />
                                    </template>
                                </v-text-field>

                                <!-- Password Field -->
                                <v-text-field
                                    v-model="credentials.password"
                                    :type="showPassword ? 'text' : 'password'"
                                    :label="t('auth.login.password')"
                                    variant="outlined"
                                    :rules="passwordRules"
                                    :error-messages="fieldErrors.password"
                                    :disabled="authStore.isLoading"
                                    class="mb-4"
                                    @input="clearFieldError('password')"
                                    @keyup.enter="handleLogin"
                                >
                                    <template #prepend-inner>
                                        <SmartIcon
                                            icon="lock"
                                            size="sm"
                                            class="mr-2"
                                        />
                                    </template>
                                    <template #append-inner>
                                        <SmartIcon
                                            :icon="showPassword ? 'eye' : 'eye-off'"
                                            size="sm"
                                            class="cursor-pointer"
                                            @click="showPassword = !showPassword"
                                        />
                                    </template>
                                </v-text-field>

                                <!-- Error Alert -->
                                <v-alert
                                    v-if="authStore.error"
                                    type="error"
                                    variant="tonal"
                                    class="mb-4"
                                    :text="authStore.error"
                                    closable
                                    @click:close="authStore.clearError"
                                />

                                <!-- Login Button -->
                                <v-btn
                                    type="submit"
                                    block
                                    size="large"
                                    color="primary"
                                    :loading="authStore.isLoading"
                                    :disabled="!formValid"
                                    class="mb-4"
                                >
                                    <template #prepend>
                                        <SmartIcon
                                            icon="log-in"
                                            size="sm"
                                        />
                                    </template>
                                    {{
                                        authStore.isLoading
                                            ? t('auth.login.loading')
                                            : t('auth.login.submit')
                                    }}
                                </v-btn>

                                <!-- Forgot Password Link -->
                                <div class="text-center">
                                    <v-btn
                                        variant="text"
                                        color="primary"
                                        size="small"
                                        @click="showForgotPassword"
                                    >
                                        {{ t('auth.login.forgot_password') }}
                                    </v-btn>
                                </div>
                            </v-form>
                        </v-card-text>
                    </v-card>

                    <!-- Forgot Password Snackbar -->
                    <v-snackbar
                        v-model="forgotPasswordSnackbar"
                        timeout="4000"
                        color="info"
                    >
                        {{ t('auth.login.forgot_password_message') }}
                        <template v-slot:actions>
                            <v-btn
                                color="white"
                                variant="text"
                                @click="forgotPasswordSnackbar = false"
                            >
                                Close
                            </v-btn>
                        </template>
                    </v-snackbar>
                </v-col>
            </v-row>
        </v-container>
    </v-main>
</template>

<script setup lang="ts">
    import { ref, reactive, onMounted } from 'vue'
    import { useRouter } from 'vue-router'
    import { useAuthStore } from '@/stores/auth'
    import { useTranslations } from '@/composables/useTranslations'
    import LanguageSwitch from '@/components/common/LanguageSwitch.vue'
    import SmartIcon from '@/components/common/SmartIcon.vue'

    // Router, store, and translations
    const router = useRouter()
    const authStore = useAuthStore()
    const { t, translateError } = useTranslations()

    // Form data
    const loginForm = ref()
    const formValid = ref(false)
    const showPassword = ref(false)
    const forgotPasswordSnackbar = ref(false)

    // Form state
    const credentials = reactive({
        username: '',
        password: '',
    })

    // Field-specific errors
    const fieldErrors = reactive({
        username: [],
        password: [],
    })

    // Validation rules with translations
    const usernameRules = [
        (v: unknown) => !!v,
        (v: unknown) => {
            const isValid = v && typeof v === 'string' && v.length >= 3
            return isValid ?? t('auth.login.errors.username_too_short')
        },
    ]

    const passwordRules = [
        (v: unknown) => !!v,
        (v: unknown) => {
            const isValid = v && typeof v === 'string' && v.length >= 8
            return isValid ?? t('auth.login.errors.password_too_short')
        },
    ]

    // Methods
    const handleLogin = async () => {
        if (!formValid.value) {
            return
        }

        // Clear previous field errors
        fieldErrors.username = []
        fieldErrors.password = []

        try {
            await authStore.login(credentials)

            // Redirect to intended page or dashboard on successful login
            const redirectParam = router.currentRoute.value.query.redirect
            const redirectTo =
                (Array.isArray(redirectParam) ? redirectParam[0] : redirectParam) ?? '/dashboard'
            void router.push(redirectTo)
        } catch (error) {
            // Handle specific validation errors from backend
            const rawErrorMessage =
                error instanceof Error ? error.message : t('general.errors.unknown')
            const translatedErrorMessage = translateError(rawErrorMessage)

            console.error('[Login] Login failed:', {
                error: rawErrorMessage,
                translatedError: translatedErrorMessage,
                credentials: { username: credentials.username, password: '[REDACTED]' },
            })

            // Map backend errors to specific fields if possible
            const lowerErrorMessage = rawErrorMessage.toLowerCase()
            if (lowerErrorMessage.includes('username') || lowerErrorMessage.includes('user')) {
                fieldErrors.username = [translatedErrorMessage]
            } else if (
                lowerErrorMessage.includes('password') ||
                lowerErrorMessage.includes('credential')
            ) {
                fieldErrors.password = [translatedErrorMessage]
            }
            // General errors are handled by the auth store's error state
            // Make sure the error is visible to the user through the store
            if (
                !lowerErrorMessage.includes('username') &&
                !lowerErrorMessage.includes('password') &&
                !lowerErrorMessage.includes('user') &&
                !lowerErrorMessage.includes('credential')
            ) {
                // This is a general error, make sure it shows in the error alert
                console.log('[Login] General error will be shown in alert:', translatedErrorMessage)
            }
        }
    }

    const clearFieldError = field => {
        fieldErrors[field] = []
        authStore.clearError()
    }

    const showForgotPassword = () => {
        forgotPasswordSnackbar.value = true
    }

    // Lifecycle
    onMounted(() => {
        // If user is already authenticated, redirect to appropriate page
        if (authStore.isAuthenticated) {
            const redirectParam = router.currentRoute.value.query.redirect
            const redirectTo =
                (Array.isArray(redirectParam) ? redirectParam[0] : redirectParam) ?? '/dashboard'
            void router.push(redirectTo)
        }
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
