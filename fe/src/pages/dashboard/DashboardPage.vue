<template>
    <div>
        <PageLayout>
            <v-row>
                <!-- Entity Definitions Tile -->
                <v-col
                    cols="12"
                    md="3"
                >
                    <v-card
                        color="primary"
                        variant="tonal"
                    >
                        <v-card-text>
                            <div class="text-h6">
                                {{ t('dashboard.tiles.entity_definitions') }}
                            </div>
                            <div
                                v-if="loading"
                                class="text-h3"
                            >
                                <v-progress-circular
                                    indeterminate
                                    size="default"
                                />
                            </div>
                            <div
                                v-else
                                class="text-h3"
                            >
                                {{ stats?.entity_definitions_count ?? 0 }}
                            </div>
                        </v-card-text>
                    </v-card>
                </v-col>

                <!-- Entities Tile -->
                <v-col
                    cols="12"
                    md="3"
                >
                    <v-card
                        color="success"
                        variant="tonal"
                    >
                        <v-card-text>
                            <div class="text-h6">
                                {{ t('dashboard.tiles.entities') }}
                            </div>
                            <div
                                v-if="loading"
                                class="text-h3"
                            >
                                <v-progress-circular
                                    indeterminate
                                    size="default"
                                />
                            </div>
                            <div v-else>
                                <div class="text-h3">
                                    {{ stats?.entities?.total ?? 0 }}
                                </div>
                                <div
                                    v-if="topEntityType"
                                    class="text-caption mt-1"
                                >
                                    {{
                                        t('dashboard.tiles.top_entity_type', {
                                            type: topEntityType.entity_type,
                                            count: topEntityType.count,
                                        })
                                    }}
                                </div>
                            </div>
                        </v-card-text>
                    </v-card>
                </v-col>

                <!-- Workflows Tile -->
                <v-col
                    cols="12"
                    md="3"
                >
                    <v-card
                        color="info"
                        variant="tonal"
                    >
                        <v-card-text>
                            <div class="text-h6">
                                {{ t('dashboard.tiles.workflows') }}
                            </div>
                            <div
                                v-if="loading"
                                class="text-h3"
                            >
                                <v-progress-circular
                                    indeterminate
                                    size="default"
                                />
                            </div>
                            <div v-else>
                                <div class="text-h3">
                                    {{ stats?.workflows?.total ?? 0 }}
                                </div>
                                <div
                                    v-if="latestWorkflowStates.length > 0"
                                    class="text-caption mt-1"
                                >
                                    {{
                                        t('dashboard.tiles.latest_workflow_states', {
                                            states:
                                                latestWorkflowStates.slice(0, 3).join(', ') +
                                                (latestWorkflowStates.length > 3 ? '...' : ''),
                                        })
                                    }}
                                </div>
                            </div>
                        </v-card-text>
                    </v-card>
                </v-col>

                <!-- Online Users Tile -->
                <v-col
                    cols="12"
                    md="3"
                >
                    <v-card
                        color="warning"
                        variant="tonal"
                    >
                        <v-card-text>
                            <div class="text-h6">
                                {{ t('dashboard.tiles.online_users') }}
                            </div>
                            <div
                                v-if="loading"
                                class="text-h3"
                            >
                                <v-progress-circular
                                    indeterminate
                                    size="default"
                                />
                            </div>
                            <div
                                v-else
                                class="text-h3"
                            >
                                {{ stats?.online_users_count ?? 0 }}
                            </div>
                        </v-card-text>
                    </v-card>
                </v-col>
            </v-row>

            <!-- Quick Actions -->
            <v-row class="mt-4">
                <v-col cols="12">
                    <v-card variant="outlined">
                        <v-card-title>{{ t('dashboard.quick_actions.title') }}</v-card-title>
                        <v-card-text>
                            <v-row>
                                <v-col
                                    v-if="canAccessEntityDefinitions"
                                    cols="12"
                                    sm="6"
                                    md="auto"
                                >
                                    <v-btn
                                        color="primary"
                                        variant="outlined"
                                        block
                                        @click="$router.push('/entity-definitions?create=true')"
                                    >
                                        <template #prepend>
                                            <SmartIcon
                                                icon="plus"
                                                size="sm"
                                            />
                                        </template>
                                        {{ t('dashboard.quick_actions.new_entity_definition') }}
                                    </v-btn>
                                </v-col>
                                <v-col
                                    v-if="canAccessEntities"
                                    cols="12"
                                    sm="6"
                                    md="auto"
                                >
                                    <v-btn
                                        color="success"
                                        variant="outlined"
                                        block
                                        @click="$router.push('/entities?create=true')"
                                    >
                                        <template #prepend>
                                            <SmartIcon
                                                icon="database"
                                                size="sm"
                                            />
                                        </template>
                                        {{ t('dashboard.quick_actions.create_entity') }}
                                    </v-btn>
                                </v-col>
                                <v-col
                                    v-if="canAccessApiKeys"
                                    cols="12"
                                    sm="6"
                                    md="auto"
                                >
                                    <v-btn
                                        color="info"
                                        variant="outlined"
                                        block
                                        @click="$router.push('/api-keys?create=true')"
                                    >
                                        <template #prepend>
                                            <SmartIcon
                                                icon="key"
                                                size="sm"
                                            />
                                        </template>
                                        {{ t('dashboard.quick_actions.generate_api_key') }}
                                    </v-btn>
                                </v-col>
                                <v-col
                                    v-if="canAccessWorkflows"
                                    cols="12"
                                    sm="6"
                                    md="auto"
                                >
                                    <v-btn
                                        color="purple"
                                        variant="outlined"
                                        block
                                        @click="$router.push('/workflows?create=true')"
                                    >
                                        <template #prepend>
                                            <SmartIcon
                                                icon="git-branch"
                                                size="sm"
                                            />
                                        </template>
                                        {{ t('dashboard.quick_actions.create_workflow') }}
                                    </v-btn>
                                </v-col>
                                <v-col
                                    v-if="canAccessUsers"
                                    cols="12"
                                    sm="6"
                                    md="auto"
                                >
                                    <v-btn
                                        color="orange"
                                        variant="outlined"
                                        block
                                        @click="$router.push('/permissions?tab=users&create=true')"
                                    >
                                        <template #prepend>
                                            <SmartIcon
                                                icon="user-plus"
                                                size="sm"
                                            />
                                        </template>
                                        {{ t('dashboard.quick_actions.users') }}
                                    </v-btn>
                                </v-col>
                            </v-row>
                        </v-card-text>
                    </v-card>
                </v-col>
            </v-row>
        </PageLayout>
    </div>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted } from 'vue'
    import { useAuthStore } from '@/stores/auth'
    import { typedHttpClient } from '@/api/typed-client'
    import { useTranslations } from '@/composables/useTranslations'
    import PageLayout from '@/components/layouts/PageLayout.vue'
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import type { DashboardStats } from '@/api/clients/meta'

    const authStore = useAuthStore()
    const { t } = useTranslations()

    // Permission checks for quick action buttons
    const canAccessEntityDefinitions = computed(() =>
        authStore.canAccessRoute('/entity-definitions')
    )
    const canAccessEntities = computed(() => authStore.canAccessRoute('/entities'))
    const canAccessApiKeys = computed(() => authStore.canAccessRoute('/api-keys'))
    const canAccessWorkflows = computed(() => authStore.canAccessRoute('/workflows'))
    const canAccessUsers = computed(() => authStore.canAccessRoute('/permissions'))

    // Dashboard stats
    const loading = ref(true)
    const stats = ref<DashboardStats>({
        entity_definitions_count: 0,
        entities: {
            total: 0,
            by_type: [],
        },
        workflows: {
            total: 0,
            workflows: [],
        },
        online_users_count: 0,
    })

    // Computed properties for display
    const topEntityType = computed(() => {
        if (!stats.value?.entities?.by_type || stats.value.entities.by_type.length === 0) {
            return null
        }
        return stats.value.entities.by_type[0]
    })

    const latestWorkflowStates = computed(() => {
        if (!stats.value?.workflows?.workflows) {
            return []
        }
        return stats.value.workflows.workflows
            .filter(w => w.latest_status)
            .map(w => `${w.name}: ${w.latest_status}`)
            .slice(0, 5)
    })

    // Check if user has permission to view dashboard stats
    const canViewDashboardStats = computed(() => authStore.hasPermission('dashboard_stats', 'read'))

    // Fetch dashboard stats
    const loadDashboardStats = async () => {
        // Only load if user has permission
        if (!canViewDashboardStats.value) {
            loading.value = false
            return
        }

        loading.value = true
        try {
            const data = await typedHttpClient.getDashboardStats()
            stats.value = data
        } catch (error) {
            console.error('Failed to load dashboard stats:', error)
            // Keep default values (all zeros) on error
        } finally {
            loading.value = false
        }
    }

    onMounted(() => {
        void loadDashboardStats()
    })
</script>

<style scoped>
    /* Component-specific styles */
</style>
