import { defineComponent, PropType } from 'vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'

export default defineComponent({
    name: 'DismissableBanner',
    components: {
        SmartIcon,
    },
    props: {
        show: { type: Boolean, required: true },
        message: { type: String, required: true },
        dismissLabel: { type: String, required: true },
        type: { type: String as PropType<'warning' | 'info' | 'error' | 'success'>, default: 'warning' },
        icon: { type: String, default: 'alert-triangle' },
    },
    emits: ['dismiss'],
    setup(_, { emit }) {
        return { handleDismiss: () => { emit('dismiss') } }
    },
})
