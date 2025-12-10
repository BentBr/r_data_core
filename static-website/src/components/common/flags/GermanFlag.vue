<template>
    <svg
        :width="width"
        :height="height"
        viewBox="0 0 5 3"
        xmlns="http://www.w3.org/2000/svg"
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
            fill="#dd0000"
        />
        <rect
            width="5"
            height="1"
            y="2"
            fill="#ffce00"
        />
    </svg>
</template>

<script setup lang="ts">
    defineProps<{
        width?: number
        height?: number
    }>()
</script>
