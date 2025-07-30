<template>
    <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 5 3"
        :width="width"
        :height="height"
        class="flag-svg"
    >
        <rect
            width="5"
            height="3"
            fill="#000"
        />
        <rect
            width="5"
            height="2"
            y="1"
            fill="#D00"
        />
        <rect
            width="5"
            height="1"
            y="2"
            fill="#FFCE00"
        />
    </svg>
</template>

<script setup lang="ts">
    interface Props {
        width?: number
        height?: number
    }

    withDefaults(defineProps<Props>(), {
        width: 24,
        height: 18,
    })
</script>

<style scoped>
    .flag-svg {
        border-radius: 2px;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
        display: block;
    }
</style>
