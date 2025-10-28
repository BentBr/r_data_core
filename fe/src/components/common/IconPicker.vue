<template>
    <div>
        <v-text-field
            v-model="searchQuery"
            :label="label"
            prepend-inner-icon="mdi-magnify"
            clearable
            placeholder="Search icons..."
            @update:model-value="filterIcons"
        />

        <v-card
            v-if="showPicker"
            class="mt-2"
            max-height="300px"
            style="overflow-y: auto"
        >
            <v-card-text class="pa-2">
                <div class="d-flex flex-wrap">
                    <v-btn
                        v-for="icon in filteredIcons"
                        :key="icon"
                        :icon="icon"
                        variant="text"
                        size="small"
                        class="ma-1"
                        :color="selectedIcon === icon ? 'primary' : undefined"
                        @click="selectIcon(icon)"
                    />
                </div>
            </v-card-text>
        </v-card>

        <div
            v-if="selectedIcon"
            class="d-flex align-center mt-2"
        >
            <v-icon
                :icon="selectedIcon"
                class="mr-2"
                size="24"
            />
            <span class="text-body-2">{{ selectedIcon }}</span>
        </div>
    </div>
</template>

<script setup lang="ts">
    import { ref, computed, watch } from 'vue'

    interface Props {
        modelValue?: string
        label?: string
        showPicker?: boolean
    }

    interface Emits {
        (e: 'update:modelValue', value: string): void
    }

    const props = withDefaults(defineProps<Props>(), {
        label: 'Select Icon',
        showPicker: true,
    })

    const emit = defineEmits<Emits>()

    const searchQuery = ref('')
    const selectedIcon = ref(props.modelValue ?? '')

    // Common Material Design Icons
    const commonIcons = [
        'mdi-file-document',
        'mdi-folder',
        'mdi-folder-tree',
        'mdi-database',
        'mdi-table',
        'mdi-account',
        'mdi-account-group',
        'mdi-account-multiple',
        'mdi-calendar',
        'mdi-calendar-clock',
        'mdi-clock',
        'mdi-clock-outline',
        'mdi-tag',
        'mdi-tag-multiple',
        'mdi-label',
        'mdi-label-outline',
        'mdi-text',
        'mdi-text-box',
        'mdi-text-box-outline',
        'mdi-numeric',
        'mdi-numeric-0-box',
        'mdi-numeric-1-box',
        'mdi-numeric-2-box',
        'mdi-numeric-3-box',
        'mdi-numeric-4-box',
        'mdi-numeric-5-box',
        'mdi-numeric-6-box',
        'mdi-numeric-7-box',
        'mdi-numeric-8-box',
        'mdi-numeric-9-box',
        'mdi-checkbox-marked',
        'mdi-checkbox-blank-outline',
        'mdi-toggle-switch',
        'mdi-toggle-switch-off',
        'mdi-image',
        'mdi-image-outline',
        'mdi-file',
        'mdi-file-outline',
        'mdi-file-pdf-box',
        'mdi-file-word-box',
        'mdi-file-excel-box',
        'mdi-file-powerpoint-box',
        'mdi-link',
        'mdi-link-variant',
        'mdi-email',
        'mdi-email-outline',
        'mdi-phone',
        'mdi-phone-outline',
        'mdi-map-marker',
        'mdi-map-marker-outline',
        'mdi-home',
        'mdi-home-outline',
        'mdi-cog',
        'mdi-cog-outline',
        'mdi-settings',
        'mdi-settings-outline',
        'mdi-tools',
        'mdi-wrench',
        'mdi-wrench-outline',
        'mdi-hammer',
        'mdi-hammer-wrench',
        'mdi-screwdriver',
        'mdi-palette',
        'mdi-palette-outline',
        'mdi-brush',
        'mdi-brush-outline',
        'mdi-pencil',
        'mdi-pencil-outline',
        'mdi-pen',
        'mdi-pen-off',
        'mdi-eraser',
        'mdi-eraser-variant',
        'mdi-marker',
        'mdi-marker-cancel',
        'mdi-highlighter',
        'mdi-format-paint',
        'mdi-format-color-fill',
        'mdi-format-color-text',
        'mdi-format-size',
        'mdi-format-bold',
        'mdi-format-italic',
        'mdi-format-underline',
        'mdi-format-strikethrough',
        'mdi-format-align-left',
        'mdi-format-align-center',
        'mdi-format-align-right',
        'mdi-format-align-justify',
        'mdi-format-list-bulleted',
        'mdi-format-list-numbered',
        'mdi-format-list-checks',
        'mdi-format-list-text',
        'mdi-format-quote-close',
        'mdi-format-quote-open',
        'mdi-format-header-1',
        'mdi-format-header-2',
        'mdi-format-header-3',
        'mdi-format-header-4',
        'mdi-format-header-5',
        'mdi-format-header-6',
        'mdi-format-paragraph',
        'mdi-format-line-spacing',
        'mdi-format-line-height',
        'mdi-format-letter-spacing',
        'mdi-format-letter-case',
        'mdi-format-letter-case-lower',
        'mdi-format-letter-case-upper',
        'mdi-format-letter-matches',
        'mdi-format-rotate-90',
        'mdi-format-rotate-180',
        'mdi-format-rotate-270',
        'mdi-format-vertical-align-top',
        'mdi-format-vertical-align-center',
        'mdi-format-vertical-align-bottom',
        'mdi-format-horizontal-align-left',
        'mdi-format-horizontal-align-center',
        'mdi-format-horizontal-align-right',
        'mdi-format-horizontal-align-justify',
        'mdi-format-indent-increase',
        'mdi-format-indent-decrease',
        'mdi-format-clear',
        'mdi-format-color-highlight',
        'mdi-format-page-break',
        'mdi-format-columns',
        'mdi-format-wrap-square',
        'mdi-format-wrap-tight',
        'mdi-format-wrap-top-bottom',
        'mdi-format-wrap-inline',
        'mdi-format-wrap-text',
        'mdi-format-wrap-text-inverse',
        'mdi-format-wrap-text-square',
        'mdi-format-wrap-text-tight',
        'mdi-format-wrap-text-top-bottom',
        'mdi-format-wrap-text-inline',
        'mdi-format-wrap-text-inverse-square',
        'mdi-format-wrap-text-inverse-tight',
        'mdi-format-wrap-text-inverse-top-bottom',
        'mdi-format-wrap-text-inverse-inline',
        'mdi-format-wrap-text-square-inverse',
        'mdi-format-wrap-text-tight-inverse',
        'mdi-format-wrap-text-top-bottom-inverse',
        'mdi-format-wrap-text-inline-inverse',
        'mdi-format-wrap-text-square-tight',
        'mdi-format-wrap-text-square-top-bottom',
        'mdi-format-wrap-text-square-inline',
        'mdi-format-wrap-text-tight-top-bottom',
        'mdi-format-wrap-text-tight-inline',
        'mdi-format-wrap-text-top-bottom-inline',
        'mdi-format-wrap-text-square-tight-top-bottom',
        'mdi-format-wrap-text-square-tight-inline',
        'mdi-format-wrap-text-square-top-bottom-inline',
        'mdi-format-wrap-text-tight-top-bottom-inline',
        'mdi-format-wrap-text-square-tight-top-bottom-inline',
    ]

    const filteredIcons = computed(() => {
        if (!searchQuery.value) {
            return commonIcons
        }
        return commonIcons.filter(icon =>
            icon.toLowerCase().includes(searchQuery.value.toLowerCase())
        )
    })

    const filterIcons = () => {
        // This is handled by the computed property
    }

    const selectIcon = (icon: string) => {
        selectedIcon.value = icon
        emit('update:modelValue', icon)
    }

    watch(
        () => props.modelValue,
        newValue => {
            selectedIcon.value = newValue ?? ''
        }
    )
</script>
