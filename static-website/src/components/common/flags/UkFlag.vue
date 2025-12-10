<template>
    <svg
        :width="width"
        :height="height"
        viewBox="0 0 60 30"
        xmlns="http://www.w3.org/2000/svg"
    >
        <clipPath id="a">
            <path d="M0 0v30h60V0z" />
        </clipPath>
        <g clip-path="url(#a)">
            <path
                d="M0 0h60v30H0z"
                fill="#012169"
            />
            <path
                d="m0 0 60 30m0-30L0 30"
                stroke="#fff"
                stroke-width="6"
            />
            <path
                d="m0 0 60 30m0-30L0 30"
                stroke="#c8102e"
                stroke-width="4"
            />
            <path
                d="M30 0v30M0 15h60"
                stroke="#fff"
                stroke-width="10"
            />
            <path
                d="M30 0v30M0 15h60"
                stroke="#c8102e"
                stroke-width="6"
            />
        </g>
    </svg>
</template>

<script setup lang="ts">
    defineProps<{
        width?: number
        height?: number
    }>()
</script>
