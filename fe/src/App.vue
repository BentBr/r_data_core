<template>
    <v-app>
        <component :is="layoutComponent">
            <router-view />
        </component>
    </v-app>
</template>

<script setup lang="ts">
    import { computed, onMounted } from 'vue'
    import { useRoute } from 'vue-router'
    import MainLayout from '@/layouts/MainLayout.vue'
    import LoginLayout from '@/layouts/LoginLayout.vue'
    import { useAuthStore } from '@/stores/auth'
    import { useTheme } from '@/composables/useTheme'

    const route = useRoute()
    const authStore = useAuthStore()

    // Initialize theme system
    const {} = useTheme()

    // Determine which layout to use based on route and auth status
    const layoutComponent = computed(() => {
        // Login page uses login layout
        if (route.name === 'Login' || !authStore.isAuthenticated) {
            return LoginLayout
        }

        // All other authenticated pages use the main layout
        return MainLayout
    })
</script>

<style scoped>
    /* App-specific styles */
</style>
