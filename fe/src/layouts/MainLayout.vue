<template>
    <div class="main-layout">
        <!-- Navigation Drawer -->
        <v-navigation-drawer
            v-model="drawer"
            app
            :permanent="!isMobile"
            :temporary="isMobile"
            :scrim="isMobile"
            width="280"
        >
            <!-- App Title -->
            <v-list-item
                title="R Data Core"
                subtitle="Admin Interface"
                nav
            >
                <template #prepend>
                    <SmartIcon
                        icon="database"
                        :size="24"
                        class="mr-3"
                    />
                </template>
            </v-list-item>

            <v-divider />

            <!-- Navigation Items -->
            <v-list nav>
                <v-list-item
                    v-for="item in navigationItems"
                    :key="item.path"
                    :title="item.title"
                    :value="item.path"
                    :to="item.path"
                    router
                >
                    <template #prepend>
                        <SmartIcon
                            :icon="item.icon"
                            :size="24"
                            class="mr-3"
                        />
                    </template>
                </v-list-item>
            </v-list>
        </v-navigation-drawer>

        <!-- App Bar -->
        <v-app-bar app>
            <v-app-bar-nav-icon
                variant="text"
                @click.stop="toggleNav"
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
            <DefaultPasswordBanner />
            <MobileWarningBanner />
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
    import { ref, computed, onMounted, onUnmounted } from 'vue'
    import { useRoute } from 'vue-router'
    import { useAuthStore } from '@/stores/auth'
    import { useTranslations } from '@/composables/useTranslations'
    import LanguageSwitch from '@/components/common/LanguageSwitch.vue'
    import UserProfileMenu from '@/components/common/UserProfileMenu.vue'
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import DefaultPasswordBanner from '@/components/common/DefaultPasswordBanner.vue'
    import MobileWarningBanner from '@/components/common/MobileWarningBanner.vue'

    const route = useRoute()
    const authStore = useAuthStore()
    const { t } = useTranslations()

    // State
    const isMobile = ref(false)
    const drawer = ref(true)

    // Theme is now handled by UserProfileMenu component

    // Navigation items with translations
    const navigationItems = computed(() => {
        const items = [
            {
                title: t('navigation.dashboard'),
                icon: 'layout-dashboard',
                path: '/dashboard',
            },
            {
                title: t('navigation.entity_definitions'),
                icon: 'folder-tree',
                path: '/entity-definitions',
            },
            {
                title: t('navigation.entities'),
                icon: 'database',
                path: '/entities',
            },
            {
                title: t('navigation.api_keys'),
                icon: 'key',
                path: '/api-keys',
            },
            {
                title: t('navigation.workflows'),
                icon: 'workflow',
                path: '/workflows',
            },
            {
                title: t('navigation.permissions'),
                icon: 'shield',
                path: '/permissions',
            },
            {
                title: t('navigation.system'),
                icon: 'settings',
                path: '/system',
            },
        ]

        // Filter items based on user permissions
        return items.filter(item => {
            // Dashboard is always accessible if authenticated
            if (item.path === '/dashboard') {
                return true
            }
            // Check if user can access the route
            return authStore.canAccessRoute(item.path)
        })
    })

    // Page title
    const currentPageTitle = computed(() => {
        const currentItem = navigationItems.value.find(item => item.path === route.path)
        return currentItem?.title ?? 'R Data Core'
    })

    const updateIsMobile = () => {
        isMobile.value = window.innerWidth < 1200
        // Default drawer open on desktop, closed on mobile
        if (isMobile.value) {
            drawer.value = false
        } else {
            drawer.value = true
        }
    }

    const toggleNav = () => {
        drawer.value = !drawer.value
    }

    // Initialize
    onMounted(() => {
        updateIsMobile()
        window.addEventListener('resize', updateIsMobile)
    })

    onUnmounted(() => {
        window.removeEventListener('resize', updateIsMobile)
    })
</script>

<style scoped>
    .main-layout {
        height: 100vh;
        width: 100%;
        overflow-y: scroll; /* Always show scrollbar to prevent layout shift */
    }

    .v-navigation-drawer {
        transition: width 0.3s ease;
    }
</style>

<style>
    /* Global styles for v-main to prevent scrollbar layout shifts */
    .v-main {
        scrollbar-gutter: stable; /* Reserve space for scrollbar even when not visible */
    }

    .v-main__wrap {
        scrollbar-gutter: stable; /* Reserve space for scrollbar even when not visible */
    }

    /* Ensure router-view and transition containers reserve scrollbar space */
    router-view {
        scrollbar-gutter: stable;
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
        scrollbar-gutter: stable; /* Reserve space for scrollbar during transitions */
    }
</style>
