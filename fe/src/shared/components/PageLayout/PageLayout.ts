import { defineComponent } from 'vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'

export default defineComponent({
    name: 'PageLayout',
    components: {
        SmartIcon,
    },
    props: {
        title: {
            type: String,
            default: undefined,
        },
        icon: {
            type: String,
            default: undefined,
        },
    },
})