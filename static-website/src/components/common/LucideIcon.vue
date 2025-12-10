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
    // Only import the icons we actually use for tree-shaking
    import {
        ArrowRight,
        Bell,
        Boxes,
        Check,
        ChevronDown,
        ChevronLeft,
        ChevronRight,
        ChevronsLeft,
        ChevronsRight,
        ChevronUp,
        Code2,
        Coffee,
        Database,
        FileInput,
        GitBranch,
        Heart,
        Key,
        Layers,
        Link,
        Moon,
        Server,
        Shield,
        Sun,
        Zap,
    } from 'lucide-vue-next'

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

    // Map of icon names to components
    const iconMap: Record<string, unknown> = {
        'arrow-right': ArrowRight,
        bell: Bell,
        boxes: Boxes,
        check: Check,
        'chevron-down': ChevronDown,
        'chevron-left': ChevronLeft,
        'chevron-right': ChevronRight,
        'chevrons-left': ChevronsLeft,
        'chevrons-right': ChevronsRight,
        'chevron-up': ChevronUp,
        'code-2': Code2,
        coffee: Coffee,
        database: Database,
        'file-input': FileInput,
        'git-branch': GitBranch,
        heart: Heart,
        key: Key,
        layers: Layers,
        link: Link,
        moon: Moon,
        server: Server,
        shield: Shield,
        sun: Sun,
        zap: Zap,
    }

    const iconComponent = computed(() => {
        // Get the icon component from our map
        const Icon = iconMap[props.name] as typeof Database | undefined

        if (!Icon) {
            console.warn(`Lucide icon "${props.name}" not found in icon map`)
            // Return a placeholder with valid dimensions
            const size = typeof props.size === 'number' ? props.size : 24
            return () =>
                h(
                    'span',
                    {
                        class: 'lucide-icon-placeholder',
                        style: {
                            width: `${size}px`,
                            height: `${size}px`,
                            display: 'inline-block',
                            textAlign: 'center',
                            lineHeight: `${size}px`,
                        },
                    },
                    '?'
                )
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
        const iconSize = typeof props.size === 'number' ? props.size : parseInt(props.size, 10)

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
