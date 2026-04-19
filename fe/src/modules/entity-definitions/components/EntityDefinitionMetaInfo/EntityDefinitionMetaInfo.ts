import { defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import Badge from '@/shared/components/Badge/index.vue'
import type { EntityDefinition } from '@/types/schemas'

export default defineComponent({
    name: 'EntityDefinitionMetaInfo',
    components: {
        SmartIcon,
        Badge,
    },
    props: {
        definition: {
            type: Object as PropType<EntityDefinition>,
            required: true,
        },
    },
    setup() {
        const { t } = useTranslations()

        const formatDate = (dateString?: string) => {
            if (!dateString) {
                return 'N/A'
            }
            return new Date(dateString).toLocaleDateString()
        }

        return {
            t, formatDate,
        }
    },
})
