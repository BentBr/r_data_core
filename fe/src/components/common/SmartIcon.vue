<template>
    <LucideIcon
        :name="iconName"
        :size="computedSize"
        :color="color"
        :stroke-width="strokeWidth"
    />
</template>

<script setup lang="ts">
    import { computed } from 'vue'
    import LucideIcon from './LucideIcon.vue'
    import { iconSizes } from '@/design-system/components'

    /**
     * Standard icon sizes:
     * - xs: 16px - Small inline icons, table actions
     * - sm: 20px - Button icons, form field icons
     * - md: 24px - Default size, most common use case
     * - lg: 28px - Page headers, prominent icons
     * - xl: 32px - Large display icons
     */
    interface Props {
        icon?: string
        /**
         * Icon size. Can be a number (px) or one of: 'xs' (16px), 'sm' (20px), 'md' (24px), 'lg' (28px), 'xl' (32px)
         * Default: 'md' (24px)
         */
        size?: number | string | 'xs' | 'sm' | 'md' | 'lg' | 'xl'
        color?: string
        strokeWidth?: number | string
        tag?: string
    }

    const props = withDefaults(defineProps<Props>(), {
        icon: 'file-text',
        size: 'md',
        strokeWidth: 2,
        tag: 'span',
    })

    const iconName = computed(() => {
        let icon = props.icon ?? 'file-text'
        
        // Handle mdi- prefixed icons by stripping the prefix
        // These should be mapped via Vuetify icon aliases, but as a fallback,
        // we strip the prefix so they can be resolved
        if (icon.startsWith('mdi-')) {
            icon = icon.replace('mdi-', '')
        }
        
        return icon
    })

    const computedSize = computed(() => {
        if (typeof props.size === 'string' && props.size in iconSizes) {
            return iconSizes[props.size as keyof typeof iconSizes]
        }
        return props.size
    })
</script>
