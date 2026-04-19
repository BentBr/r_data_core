import { computed, defineComponent } from 'vue'
import LucideIcon from '../LucideIcon/index.vue'
import { iconSizes } from '@/design-system/components'

export default defineComponent({
    name: 'SmartIcon',
    components: {
        LucideIcon,
    },
    props: {
        icon: { type: String, default: 'file-text' },
        size: { type: [Number, String], default: 'md' },
        color: { type: String, default: undefined },
        strokeWidth: { type: [Number, String], default: 2 },
        tag: { type: String, default: 'span' },
    },
    setup(props) {
        const iconName = computed(() => {
            let icon = props.icon
            if (icon.startsWith('mdi-')) icon = icon.replace('mdi-', '')
            return icon
        })
        const computedSize = computed(() => {
            if (typeof props.size === 'string' && props.size in iconSizes) {
                return iconSizes[props.size as keyof typeof iconSizes]
            }
            return props.size
        })
        return { iconName, computedSize }
    },
})
