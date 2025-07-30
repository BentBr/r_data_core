<template>
    <v-container
        fluid
        class="fill-height"
    >
        <v-row
            justify="center"
            align="center"
        >
            <v-col
                cols="12"
                sm="8"
                md="4"
            >
                <v-card>
                    <v-card-title class="text-center text-h4 pa-6">
                        R Data Core Admin
                    </v-card-title>
                    <v-card-text>
                        <v-form @submit.prevent="handleLogin">
                            <v-text-field
                                v-model="username"
                                label="Username"
                                prepend-icon="mdi-account"
                                required
                            />
                            <v-text-field
                                v-model="password"
                                label="Password"
                                type="password"
                                prepend-icon="mdi-lock"
                                required
                            />
                            <v-btn
                                type="submit"
                                color="primary"
                                block
                                class="mt-4"
                                :loading="loading"
                            >
                                Login
                            </v-btn>
                        </v-form>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>
    </v-container>
</template>

<script setup lang="ts">
    import { ref } from 'vue'
    import { useRouter } from 'vue-router'

    const router = useRouter()

    const username = ref('')
    const password = ref('')
    const loading = ref(false)

    const handleLogin = async () => {
        loading.value = true
        try {
            // TODO: Implement actual login logic
            console.log('Login attempt:', { username: username.value, password: password.value })

            // Mock login - redirect to dashboard
            setTimeout(() => {
                router.push('/dashboard')
                loading.value = false
            }, 1000)
        } catch (error) {
            console.error('Login error:', error)
            loading.value = false
        }
    }
</script>

<style scoped>
    .fill-height {
        min-height: 100vh;
    }
</style>
