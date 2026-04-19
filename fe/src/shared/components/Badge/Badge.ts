import { computed, defineComponent, PropType } from 'vue'
import { badgeConfigs, getStatusColor } from '@/design-system/components'

export default defineComponent({
    name: 'Badge',
    props: {
        color: { type: String, default: undefined },
        size: { type: String as PropType<'small' | 'default' | 'large'>, default: 'default' },
        variant: { type: String as PropType<'flat' | 'outlined' | 'text' | 'elevated' | 'tonal'>, default: badgeConfigs.variant },
        status: { type: String, default: undefined },
    },
    setup(props) {
        const computedColor = computed(() => {
            if (props.status) return getStatusColor(props.status)
            return props.color ?? badgeConfigs.status.default
        })
        const customClass = computed(() => props.status ? 'status-badge' : '')
        return { computedColor, customClass }
    },
})
