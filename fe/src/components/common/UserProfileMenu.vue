<template>
    <v-menu offset-y>
        <template v-slot:activator="{ props }">
            <v-btn
                variant="text"
                v-bind="props"
                class="text-none"
                :title="t('general.user.profile')"
            >
                <div class="d-flex align-center">
                    <v-avatar
                        size="32"
                        color="primary"
                        class="mr-2"
                    >
                        <SmartIcon
                            icon="user"
                            :size="20"
                        />
                    </v-avatar>
                    <div class="d-none d-sm-flex flex-column align-start">
                        <span class="text-body-2 font-weight-medium">{{
                            authStore.user?.username
                        }}</span>
                        <span class="text-caption text-medium-emphasis">{{
                            authStore.isSuperAdmin ? 'Super Admin' : 'User'
                        }}</span>
                    </div>
                    <SmartIcon
                        icon="chevron-down"
                        :size="16"
                        class="ml-2"
                    />
                </div>
            </v-btn>
        </template>

        <v-list min-width="200">
            <!-- User Info Header -->
            <v-list-item class="px-4 py-3">
                <template v-slot:prepend>
                    <v-avatar
                        size="40"
                        color="primary"
                    >
                        <SmartIcon
                            icon="user"
                            :size="24"
                        />
                    </v-avatar>
                </template>
                <v-list-item-title class="font-weight-medium">
                    {{ authStore.user?.username }}
                </v-list-item-title>
                <v-list-item-subtitle>
                    {{ authStore.isSuperAdmin ? 'Super Admin' : 'User' }}
                </v-list-item-subtitle>
            </v-list-item>

            <v-divider />

            <!-- Profile Option -->
            <v-list-item
                :disabled="true"
                @click="goToProfile"
            >
                <template v-slot:prepend>
                    <SmartIcon
                        icon="user-cog"
                        :size="20"
                    />
                </template>
                <v-list-item-title>{{ t('general.user.profile') }}</v-list-item-title>
                <v-list-item-subtitle>{{
                    t('general.user.profile_subtitle')
                }}</v-list-item-subtitle>
            </v-list-item>

            <!-- Theme Selection -->
            <v-list-item>
                <template v-slot:prepend>
                    <SmartIcon
                        :icon="isDark ? 'moon' : 'sun'"
                        :size="20"
                    />
                </template>
                <v-list-item-title>{{ t('general.theme.mode') }}</v-list-item-title>
                <template v-slot:append>
                    <Badge
                        size="small"
                        variant="outlined"
                        class="cursor-pointer"
                        @click="toggleTheme"
                    >
                        {{ getThemeDisplayName() }}
                    </Badge>
                </template>
            </v-list-item>

            <v-divider />

            <!-- Logout Option -->
            <v-list-item
                class="text-error"
                @click="handleLogout"
            >
                <template v-slot:prepend>
                    <SmartIcon
                        icon="log-out"
                        color="error"
                        :size="20"
                    />
                </template>
                <v-list-item-title>{{ t('navigation.logout') }}</v-list-item-title>
                <v-list-item-subtitle>{{ t('general.user.logout_subtitle') }}</v-list-item-subtitle>
            </v-list-item>
        </v-list>
    </v-menu>
</template>

<script setup lang="ts">
    import { useRouter } from 'vue-router'
    import { useAuthStore } from '@/stores/auth'
    import { useTranslations } from '@/composables/useTranslations'
    import { useTheme } from '@/composables/useTheme'
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import Badge from '@/components/common/Badge.vue'

    const router = useRouter()
    const authStore = useAuthStore()
    const { t } = useTranslations()
    const { isDark, toggleTheme, userPreference } = useTheme()

    const getThemeDisplayName = (): string => {
        switch (userPreference.value) {
            case 'system':
                return t('general.theme.system')
            case 'light':
                return t('general.theme.light')
            case 'dark':
                return t('general.theme.dark')
            default:
                return t('general.theme.system')
        }
    }

    const goToProfile = () => {
        // TODO: Implement profile page when available
        // router.push('/profile')
        console.log('Profile page not yet implemented')
    }

    const handleLogout = async () => {
        // Clear auth state immediately to prevent API calls
        authStore.clearAuthState()

        // Redirect to login page without any query parameters to prevent redirect loops
        void router.push({ name: 'Login', query: {} })

        // Then handle the full logout process
        try {
            await authStore.logout()
        } catch (err) {
            console.error('Logout failed:', err)
        }
    }
</script>

<style scoped>
    .cursor-pointer {
        cursor: pointer;
    }

    .v-list-item.text-error .v-list-item-title {
        color: rgb(var(--v-theme-error));
    }
</style>
