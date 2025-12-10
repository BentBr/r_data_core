<template>
    <v-icon
        :icon="icon"
        v-bind="$attrs"
    />
</template>

<script setup lang="ts">
    defineProps<{
        icon: string
        size?: string | number
    }>()
</script>
