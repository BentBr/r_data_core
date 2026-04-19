import { defineComponent } from 'vue'

export default defineComponent({
    name: 'GermanFlag',
    props: {
        width: { type: Number, default: 24 },
        height: { type: Number, default: 18 },
    },
})
