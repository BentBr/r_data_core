<template>
    <v-app>
        <component
            :is="layoutComponent"
            :key="layoutKey"
        >
            <router-view v-slot="{ Component, route }">
                <transition
                    name="fade"
                    mode="out-in"
                >
                    <component
                        :is="Component"
                        :key="route.path"
                    />
                </transition>
            </router-view>
        </component>
    </v-app>
</template>

<script setup lang="ts">
    import { computed } from 'vue'
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

    // Create a unique key for the layout to ensure proper transitions
    const layoutKey = computed(() => {
        return `${route.name}-${authStore.isAuthenticated}`
    })
</script>

<style scoped>
    /* Smooth fade transitions for router view only */
    .fade-enter-active,
    .fade-leave-active {
        transition: opacity 0.5s ease;
    }

    .fade-enter-from,
    .fade-leave-to {
        opacity: 0;
    }

    .fade-enter-to,
    .fade-leave-from {
        opacity: 1;
    }
</style>
