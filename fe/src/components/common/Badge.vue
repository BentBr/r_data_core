<template>
    <v-chip
        :color="computedColor"
        :size="size"
        :variant="variant"
        :class="customClass"
    >
        <slot />
    </v-chip>
</template>

<script setup lang="ts">
    import { computed } from 'vue'
    import { badgeConfigs, getStatusColor } from '@/design-system/components'

    interface Props {
        /**
         * Badge color. Can be a status string (success, error, warning, info) or a Vuetify color name
         */
        color?: string
        /**
         * Badge size
         */
        size?: 'small' | 'default' | 'large'
        /**
         * Badge variant
         */
        variant?: 'flat' | 'outlined' | 'text' | 'elevated' | 'tonal'
        /**
         * Status string for automatic color mapping (e.g., 'success', 'error', 'warning', 'info')
         */
        status?: string
    }

    const props = withDefaults(defineProps<Props>(), {
        size: 'default',
        variant: badgeConfigs.variant,
    })

    const computedColor = computed(() => {
        // If status is provided, use it to determine color
        if (props.status) {
            return getStatusColor(props.status)
        }
        // Otherwise use the color prop, or default to muted
        return props.color ?? badgeConfigs.status.default
    })

    const customClass = computed(() => {
        return props.status ? 'status-badge' : ''
    })
</script>

<style scoped>
    .status-badge {
        font-weight: 500;
    }
</style>
