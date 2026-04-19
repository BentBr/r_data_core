import { ref, onMounted, defineComponent } from 'vue'
import { useRoute } from 'vue-router'
import PageLayout from '@/shared/components/PageLayout/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import UserTab from '@/modules/permissions/components/page-tabs/UserTab/index.vue'
import RoleTab from '@/modules/permissions/components/page-tabs/RoleTab/index.vue'
import { useTranslations } from '@/shared/composables/useTranslations'

export default defineComponent({
    name: 'PermissionsPage',
    components: {
        PageLayout,
        SmartIcon,
        UserTab,
        RoleTab,
    },
    setup() {
        const { t } = useTranslations()
        const route = useRoute()

        const activeTab = ref('users')

        onMounted(() => {
            // Switch to appropriate tab if requested via query param
            if (route.query.tab === 'users') {
                activeTab.value = 'users'
            } else if (route.query.tab === 'roles') {
                activeTab.value = 'roles'
            }
        })

        return {
            t,
            activeTab,
        }
    },
})
