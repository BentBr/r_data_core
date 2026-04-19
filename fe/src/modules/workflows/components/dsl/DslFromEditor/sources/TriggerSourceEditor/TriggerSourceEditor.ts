import { defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { buildApiUrl } from '@/env-check'
import type { FromDef } from '../../../contracts'

export default defineComponent({
    name: 'TriggerSourceEditor',
    props: {
        modelValue: { type: Object as PropType<FromDef>, required: true },
        workflowUuid: { type: String, default: null },
    },
    setup(props) {
        const { t } = useTranslations()
        return { t, getTriggerEndpointUri: () => buildApiUrl(`/api/v1/workflows/${props.workflowUuid ?? '{uuid}'}/trigger`) }
    }
})
