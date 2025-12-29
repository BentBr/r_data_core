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
                :variant="buttonConfigs.text.variant"
                @click="closeSnackbar"
            >
                Close
            </v-btn>
        </template>
    </v-snackbar>
</template>

<script setup lang="ts">
    import { ref, computed, watch } from 'vue'
    import type { SnackbarConfig } from '@/types/schemas'
    import { buttonConfigs } from '@/design-system/components'
    import { useSnackbar } from '@/composables/useSnackbar'

    interface Props {
        snackbar: SnackbarConfig | null
    }

    const props = defineProps<Props>()
    const { clearSnackbar } = useSnackbar()

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

    // Close snackbar and clear global state after animation completes
    const closeSnackbar = () => {
        showSnackbar.value = false
        // Delay clearing global state until after close animation (~300ms)
        setTimeout(() => {
            clearSnackbar()
        }, 300)
    }

    // Watch for changes in snackbar prop to show the snackbar
    watch(
        () => props.snackbar,
        newSnackbar => {
            if (newSnackbar?.message) {
                showSnackbar.value = true
            }
        },
        { immediate: true }
    )

    // When snackbar closes by timeout, clear global state after animation
    watch(showSnackbar, newValue => {
        if (!newValue) {
            setTimeout(() => {
                clearSnackbar()
            }, 300)
        }
    })
</script>
