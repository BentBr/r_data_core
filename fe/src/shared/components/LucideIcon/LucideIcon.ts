import { computed, h, defineComponent } from 'vue'
import * as LucideIcons from 'lucide-vue-next'

export default defineComponent({
    name: 'LucideIcon',
    props: {
        name: { type: String, required: true },
        size: { type: [Number, String], default: 24 },
        color: { type: String, default: undefined },
        strokeWidth: { type: [Number, String], default: 2 },
    },
    setup(props) {
        const iconComponent = computed(() => {
            if (!props.name.trim()) return () => h('span', { style: { display: 'none' } })
            const iconName = props.name.split('-').map(part => part.charAt(0).toUpperCase() + part.slice(1)).join('')
            const Icon = (LucideIcons as Record<string, any>)[iconName]
            if (!Icon) {
                console.warn(`Lucide icon "${props.name}" (${iconName}) not found`)
                return () => h('span', { style: { display: 'none' } })
            }
            let iconColor = props.color
            if (iconColor && !iconColor.startsWith('#') && !iconColor.startsWith('rgb')) {
                const colorVar = `--v-theme-${iconColor}`
                if (typeof window !== 'undefined') {
                    const computedColor = getComputedStyle(document.documentElement).getPropertyValue(colorVar)
                    if (computedColor) iconColor = `rgb(${computedColor.trim()})`
                }
            }
            const iconSize = typeof props.size === 'number' ? props.size : parseInt(String(props.size), 10) || 24
            return () => h(Icon, { size: iconSize, color: iconColor ?? 'currentColor', strokeWidth: props.strokeWidth })
        })
        return { iconComponent }
    },
})
