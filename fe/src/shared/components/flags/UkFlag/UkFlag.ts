import { computed, defineComponent } from 'vue'

export default defineComponent({
    name: 'UkFlag',
    props: {
        width: { type: Number, default: 24 },
        height: { type: Number, default: 18 },
    },
    setup() {
        const uniqueId = computed(() => Math.random().toString(36).substr(2, 9))
        return { uniqueId }
    },
})
