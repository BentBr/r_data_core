<template>
    <div class="main-layout">
        <!-- Navigation Drawer -->
        <v-navigation-drawer
            v-model="drawer"
            :rail="rail"
            permanent
            @click="rail = false"
        >
            <!-- App Title -->
            <v-list-item
                prepend-icon="mdi-database"
                title="R Data Core"
                subtitle="Admin Interface"
                nav
            />

            <v-divider />

            <!-- Navigation Items -->
            <v-list nav>
                <v-list-item
                    v-for="item in navigationItems"
                    :key="item.path"
                    :prepend-icon="item.icon"
                    :title="item.title"
                    :value="item.path"
                    :to="item.path"
                    router
                />
            </v-list>
        </v-navigation-drawer>

        <!-- App Bar -->
        <v-app-bar>
            <v-app-bar-nav-icon
                variant="text"
                @click.stop="rail = !rail"
            />

            <v-toolbar-title>{{ currentPageTitle }}</v-toolbar-title>

            <v-spacer />

            <!-- User Profile Menu -->
            <UserProfileMenu
                v-if="authStore.user"
                class="mr-2"
            />

            <!-- Language Switch -->
            <LanguageSwitch />
        </v-app-bar>

        <!-- Main Content -->
        <v-main>
            <router-view v-slot="{ Component, route }">
                <transition
                    name="fade"
                    mode="out-in"
                    appear
                >
                    <component
                        :is="Component"
                        :key="route.fullPath"
                    />
                </transition>
            </router-view>
        </v-main>
    </div>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted } from 'vue'
    import { useRoute } from 'vue-router'
    import { useAuthStore } from '@/stores/auth'
    import { useTranslations } from '@/composables/useTranslations'
    import LanguageSwitch from '@/components/common/LanguageSwitch.vue'
    import UserProfileMenu from '@/components/common/UserProfileMenu.vue'

    const route = useRoute()
    const authStore = useAuthStore()
    const { t } = useTranslations()

    // State
    const drawer = ref(true)
    const rail = ref(false)

    // Theme is now handled by UserProfileMenu component

    // Navigation items with translations
    const navigationItems = computed(() => [
        {
            title: t('navigation.dashboard'),
            icon: 'mdi-view-dashboard',
            path: '/dashboard',
        },
        {
            title: t('navigation.entity_definitions'),
            icon: 'mdi-file-tree',
            path: '/entity-definitions',
        },
        {
            title: t('navigation.entities'),
            icon: 'mdi-database',
            path: '/entities',
        },
        {
            title: t('navigation.api_keys'),
            icon: 'mdi-key',
            path: '/api-keys',
        },
        {
            title: t('navigation.permissions'),
            icon: 'mdi-shield-account',
            path: '/permissions',
        },
        {
            title: t('navigation.system'),
            icon: 'mdi-cog',
            path: '/system',
        },
    ])

    // Page title
    const currentPageTitle = computed(() => {
        const currentItem = navigationItems.value.find(item => item.path === route.path)
        return currentItem?.title ?? 'R Data Core'
    })

    // Initialize
    onMounted(() => {
        // You could add any initialization logic here
    })
</script>

<style scoped>
    .main-layout {
        height: 100vh;
        width: 100%;
    }

    .v-navigation-drawer {
        transition: width 0.3s ease;
    }

    /* Smooth fade transitions for page content */
    .fade-enter-active,
    .fade-leave-active {
        transition: opacity 0.8s ease-in-out;
        position: relative;
    }

    .fade-enter-from,
    .fade-leave-to {
        opacity: 0;
    }

    .fade-enter-to,
    .fade-leave-from {
        opacity: 1;
    }

    /* Ensure the transition container has proper positioning */
    .fade-enter-active > *,
    .fade-leave-active > * {
        position: relative;
        width: 100%;
    }
</style>
