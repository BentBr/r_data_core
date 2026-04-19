import { ref, computed, onMounted, defineComponent } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { typedHttpClient } from '@/api/typed-client'
import { useTranslations } from '@/shared/composables/useTranslations'
import PageLayout from '@/shared/components/PageLayout/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import type { DashboardStats } from '@/api/clients/meta'

export default defineComponent({
    name: 'DashboardPage',
    components: {
        PageLayout,
        SmartIcon,
    },
    setup() {
        const authStore = useAuthStore()
        const { t } = useTranslations()

        // Permission checks for quick action buttons
        const canCreateEntityDefinition = computed(() => {
            return (
                authStore.hasPermission('EntityDefinitions', 'Create') ||
                authStore.hasPermission('EntityDefinitions', 'Admin')
            )
        })
        const canCreateEntity = computed(() => {
            return (
                authStore.hasPermission('Entities', 'Create') ||
                authStore.hasPermission('Entities', 'Admin')
            )
        })
        const canCreateApiKey = computed(() => {
            return (
                authStore.hasPermission('ApiKeys', 'Create') ||
                authStore.hasPermission('ApiKeys', 'Admin')
            )
        })
        const canCreateWorkflow = computed(() => {
            return (
                authStore.hasPermission('Workflows', 'Create') ||
                authStore.hasPermission('Workflows', 'Admin')
            )
        })
        const canCreateUser = computed(() => {
            return (
                authStore.hasPermission('Users', 'Create') || authStore.hasPermission('Users', 'Admin')
            )
        })
        const canCreateRole = computed(() => {
            return (
                authStore.hasPermission('Roles', 'Create') || authStore.hasPermission('Roles', 'Admin')
            )
        })

        const hasAnyCreatePermission = computed(() => {
            return (
                canCreateEntityDefinition.value ||
                canCreateEntity.value ||
                canCreateApiKey.value ||
                canCreateWorkflow.value ||
                canCreateUser.value ||
                canCreateRole.value
            )
        })

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
            if (stats.value.entities.by_type.length === 0) {
                return null
            }
            return stats.value.entities.by_type[0]
        })

        const latestWorkflowStates = computed(() => {
            return stats.value.workflows.workflows
                .filter(w => w.latest_status)
                .map(w => `${w.name}: ${w.latest_status}`)
                .slice(0, 5)
        })

        const canViewDashboardStats = computed(() => authStore.hasPermission('DashboardStats', 'read'))

        const loadDashboardStats = async () => {
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
            } finally {
                loading.value = false
            }
        }

        onMounted(() => {
            void loadDashboardStats()
        })

        return {
            t,
            loading,
            stats,
            topEntityType,
            latestWorkflowStates,
            canCreateEntityDefinition,
            canCreateEntity,
            canCreateApiKey,
            canCreateWorkflow,
            canCreateUser,
            canCreateRole,
            hasAnyCreatePermission,
        }
    },
})
