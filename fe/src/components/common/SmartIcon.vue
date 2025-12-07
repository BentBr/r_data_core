<template>
    <LucideIcon
        :name="iconName"
        :size="size"
        :color="color"
        :stroke-width="strokeWidth"
    />
</template>

<script setup lang="ts">
    import { computed } from 'vue'
    import LucideIcon from './LucideIcon.vue'

    interface Props {
        icon?: string
        size?: number | string
        color?: string
        strokeWidth?: number | string
        tag?: string
    }

    const props = withDefaults(defineProps<Props>(), {
        icon: 'file-text',
        size: 24,
        strokeWidth: 2,
        tag: 'span',
    })

    const iconName = computed(() => {
        return props.icon ?? 'file-text'
    })
</script>
