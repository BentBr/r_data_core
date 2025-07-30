<template>
    <svg 
        xmlns="http://www.w3.org/2000/svg" 
        viewBox="0 0 60 30" 
        :width="width" 
        :height="height"
        class="flag-svg"
    >
        <defs>
            <clipPath :id="`s-${uniqueId}`">
                <path d="M0,0 v30 h60 v-30 z"/>
            </clipPath>
            <clipPath :id="`t-${uniqueId}`">
                <path d="M30,15 h30 v15 z v15 h-30 z h-30 v-15 z v-15 h30 z"/>
            </clipPath>
        </defs>
        <g :clip-path="`url(#s-${uniqueId})`">
            <path d="M0,0 v30 h60 v-30 z" fill="#012169"/>
            <path d="M0,0 L60,30 M60,0 L0,30" stroke="#fff" stroke-width="6"/>
            <path d="M0,0 L60,30 M60,0 L0,30" :clip-path="`url(#t-${uniqueId})`" stroke="#C8102E" stroke-width="4"/>
            <path d="M30,0 v30 M0,15 h60" stroke="#fff" stroke-width="10"/>
            <path d="M30,0 v30 M0,15 h60" stroke="#C8102E" stroke-width="6"/>
        </g>
    </svg>
</template>

<script setup lang="ts">
    import { computed } from 'vue'

    interface Props {
        width?: number
        height?: number
    }

    const props = withDefaults(defineProps<Props>(), {
        width: 24,
        height: 18
    })

    const uniqueId = computed(() => Math.random().toString(36).substr(2, 9))
</script>

<style scoped>
    .flag-svg {
        border-radius: 2px;
        box-shadow: 0 1px 3px rgba(0,0,0,0.2);
        display: block;
    }
</style>