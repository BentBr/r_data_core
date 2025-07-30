<template>
    <v-app>
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
                                <div class="text-h4 font-weight-bold primary--text">
                                    R Data Core
                                </div>
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
                                        label="Username"
                                        prepend-inner-icon="mdi-account"
                                        variant="outlined"
                                        :rules="usernameRules"
                                        :error-messages="fieldErrors.username"
                                        :disabled="authStore.isLoading"
                                        class="mb-4"
                                        autofocus
                                        @input="clearFieldError('username')"
                                    />

                                    <!-- Password Field -->
                                    <v-text-field
                                        v-model="credentials.password"
                                        :type="showPassword ? 'text' : 'password'"
                                        label="Password"
                                        prepend-inner-icon="mdi-lock"
                                        :append-inner-icon="
                                            showPassword ? 'mdi-eye' : 'mdi-eye-off'
                                        "
                                        variant="outlined"
                                        :rules="passwordRules"
                                        :error-messages="fieldErrors.password"
                                        :disabled="authStore.isLoading"
                                        class="mb-4"
                                        @click:append-inner="showPassword = !showPassword"
                                        @input="clearFieldError('password')"
                                        @keyup.enter="handleLogin"
                                    />

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
                                        :disabled="!formValid || authStore.isLoading"
                                        class="mb-4"
                                    >
                                        <v-icon start>mdi-login</v-icon>
                                        Sign In
                                    </v-btn>

                                    <!-- Forgot Password Link -->
                                    <div class="text-center">
                                        <v-btn
                                            variant="text"
                                            color="primary"
                                            size="small"
                                            @click="showForgotPassword"
                                        >
                                            Forgot your password?
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
                            Password reset functionality is not yet implemented.
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
    </v-app>
</template>

<script setup lang="ts">
    import { ref, reactive, onMounted } from 'vue'
    import { useRouter } from 'vue-router'
    import { useAuthStore } from '@/stores/auth'

    // Router and store
    const router = useRouter()
    const authStore = useAuthStore()

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

    // Validation rules
    const usernameRules = [
        v => !!v || 'Username is required',
        v => (v && v.length >= 3) || 'Username must be at least 3 characters',
    ]

    const passwordRules = [
        v => !!v || 'Password is required',
        v => (v && v.length >= 8) || 'Password must be at least 8 characters',
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
            const redirectTo = router.currentRoute.value.query.redirect || '/dashboard'
            router.push(redirectTo)
        } catch (error) {
            // Handle specific validation errors from backend
            const errorMessage = error instanceof Error ? error.message : 'Login failed'

            // Map backend errors to specific fields if possible
            if (errorMessage.toLowerCase().includes('username')) {
                fieldErrors.username = [errorMessage]
            } else if (errorMessage.toLowerCase().includes('password')) {
                fieldErrors.password = [errorMessage]
            }
            // General errors are handled by the auth store's error state
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
            const redirectTo = router.currentRoute.value.query.redirect || '/dashboard'
            router.push(redirectTo)
        }
    })
</script>

<style scoped>
    .fill-height {
        min-height: 100vh;
    }

    .v-container {
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    }

    .v-card {
        background: rgba(255, 255, 255, 0.95);
        backdrop-filter: blur(10px);
    }

    .v-card-title .text-h4 {
        background: linear-gradient(45deg, #1976d2, #42a5f5);
        background-clip: text;
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
    }
</style>
