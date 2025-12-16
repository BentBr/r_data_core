<template>
    <component
        :is="iconComponent"
        :size="size"
        :color="color"
        :stroke-width="strokeWidth"
        class="lucide-icon"
    />
</template>

<script setup lang="ts">
    import { computed, h } from 'vue'
    import * as LucideIcons from 'lucide-vue-next'

    interface Props {
        name: string
        size?: number | string
        color?: string
        strokeWidth?: number | string
    }

    const props = withDefaults(defineProps<Props>(), {
        size: 24,
        strokeWidth: 2,
    })

    const iconComponent = computed(() => {
        // Handle empty or whitespace-only names - render nothing
        if (!props.name?.trim()) {
            return () => h('span', { style: { display: 'none' } })
        }

        // Convert kebab-case to PascalCase (e.g., "file-document" -> "FileDocument")
        const iconName = props.name
            .split('-')
            .map(part => part.charAt(0).toUpperCase() + part.slice(1))
            .join('')

        // Get the icon component from Lucide
        const Icon = (LucideIcons as Record<string, unknown>)[iconName] as
            | typeof LucideIcons.File
            | undefined

        if (!Icon) {
            console.warn(`Lucide icon "${props.name}" (${iconName}) not found`)
            // Return nothing instead of placeholder for missing icons
            return () => h('span', { style: { display: 'none' } })
        }

        // Convert Vuetify color names to CSS color values
        let iconColor = props.color
        if (iconColor && !iconColor.startsWith('#') && !iconColor.startsWith('rgb')) {
            // Try to get CSS variable for Vuetify theme colors
            const colorVar = `--v-theme-${iconColor}`
            if (typeof window !== 'undefined') {
                const computedColor = getComputedStyle(document.documentElement).getPropertyValue(
                    colorVar
                )
                if (computedColor) {
                    iconColor = `rgb(${computedColor.trim()})`
                }
            }
        }

        // Ensure size is a valid number
        const iconSize =
            typeof props.size === 'number'
                ? props.size
                : typeof props.size === 'string'
                  ? parseInt(props.size, 10) || 24
                  : 24

        return () =>
            h(Icon, {
                size: iconSize,
                color: iconColor ?? 'currentColor',
                strokeWidth: props.strokeWidth,
            })
    })
</script>

<style scoped>
    .lucide-icon {
        display: inline-flex;
        align-items: center;
        justify-content: center;
    }
</style>
