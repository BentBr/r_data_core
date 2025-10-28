<template>
    <v-snackbar
        v-model="showSnackbar"
        :color="currentSnackbar.color"
        :timeout="currentSnackbar.timeout"
        :persistent="currentSnackbar.persistent"
    >
        {{ currentSnackbar.message }}

        <template #actions>
            <v-btn
                color="white"
                variant="text"
                @click="showSnackbar = false"
            >
                Close
            </v-btn>
        </template>
    </v-snackbar>
</template>

<script setup lang="ts">
    import { ref, computed, watch } from 'vue'
    import type { SnackbarConfig } from '@/types/schemas'

    interface Props {
        snackbar: SnackbarConfig | null
    }

    const props = defineProps<Props>()

    const showSnackbar = ref(false)

    const currentSnackbar = computed(() => {
        if (!props.snackbar) {
            return {
                message: '',
                color: 'info',
                timeout: 3000,
                persistent: false,
            }
        }
        return {
            message: props.snackbar.message,
            color: props.snackbar.color ?? 'info',
            timeout: props.snackbar.timeout ?? 3000,
            persistent: props.snackbar.persistent ?? false,
        }
    })

    // Watch for changes in snackbar prop to show the snackbar
    watch(
        () => props.snackbar,
        newSnackbar => {
            if (newSnackbar) {
                showSnackbar.value = true
            }
        },
        { immediate: true }
    )
</script>
