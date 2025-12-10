<template>
    <v-btn
        :icon="isDark ? 'mdi-white-balance-sunny' : 'mdi-weather-night'"
        variant="text"
        size="small"
        :aria-label="isDark ? 'Switch to light mode' : 'Switch to dark mode'"
        @click="toggleTheme"
    >
        <SmartIcon :icon="isDark ? 'sun' : 'moon'" />
    </v-btn>
</template>

<script setup lang="ts">
    import { computed, onMounted } from 'vue'
    import { useTheme } from 'vuetify'
    import SmartIcon from './SmartIcon.vue'

    const theme = useTheme()

    const isDark = computed(() => theme.global.current.value.dark)

    const toggleTheme = () => {
        const newTheme = isDark.value ? 'light' : 'dark'
        theme.toggle([newTheme])
        // Store preference in localStorage
        localStorage.setItem('theme-preference', newTheme)
    }

    // Load theme preference on mount
    onMounted(() => {
        const savedTheme = localStorage.getItem('theme-preference')
        if (savedTheme === 'dark' || savedTheme === 'light') {
            theme.toggle([savedTheme])
        }
    })
</script>
