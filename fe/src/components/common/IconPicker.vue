<template>
    <div class="icon-picker">
        <v-text-field
            v-model="searchQuery"
            :label="label"
            clearable
            placeholder="Search icons..."
            @update:model-value="filterIcons"
        >
            <template #prepend-inner>
                <SmartIcon
                    icon="search"
                    :size="20"
                    class="mr-2"
                />
            </template>
        </v-text-field>

        <v-card
            v-if="showPicker"
            class="mt-2 icon-picker-card"
        >
            <v-card-text class="pa-2">
                <div class="d-flex flex-wrap icon-grid">
                    <v-btn
                        v-for="icon in filteredIcons"
                        :key="icon"
                        variant="text"
                        size="small"
                        class="icon-button"
                        :color="selectedIcon === icon ? 'primary' : undefined"
                        @click="selectIcon(icon)"
                    >
                        <LucideIcon
                            :name="icon"
                            :size="20"
                        />
                    </v-btn>
                </div>
            </v-card-text>
        </v-card>

        <div
            v-if="selectedIcon"
            class="d-flex align-center mt-2"
        >
            <LucideIcon
                :name="selectedIcon"
                :size="24"
                class="mr-2"
            />
            <span class="text-body-2">{{ selectedIcon }}</span>
        </div>
    </div>
</template>

<script setup lang="ts">
    import { ref, computed, watch } from 'vue'
    import LucideIcon from './LucideIcon.vue'
    import SmartIcon from './SmartIcon.vue'

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

    // Curated list of ~500 most popular/useful Lucide icons
    const popularIcons = [
        // Files & Documents
        'file',
        'file-text',
        'file-edit',
        'file-plus',
        'file-minus',
        'file-x',
        'file-check',
        'file-search',
        'file-code',
        'file-image',
        'file-video',
        'file-audio',
        'file-type',
        'folder',
        'folder-open',
        'folder-plus',
        'folder-minus',
        'folder-x',
        'folder-check',
        'folder-search',
        'folder-tree',
        'folder-symlink',
        // Database & Data
        'database',
        'table',
        'table-2',
        'columns',
        'rows',
        'layout-grid',
        'layout-list',
        // Users & People
        'user',
        'user-plus',
        'user-minus',
        'user-x',
        'user-check',
        'users',
        'user-circle',
        'user-square',
        'user-cog',
        'user-search',
        // Navigation & Arrows
        'arrow-up',
        'arrow-down',
        'arrow-left',
        'arrow-right',
        'arrow-up-down',
        'arrow-left-right',
        'chevron-up',
        'chevron-down',
        'chevron-left',
        'chevron-right',
        'chevrons-up',
        'chevrons-down',
        'chevrons-left',
        'chevrons-right',
        'move',
        'move-up',
        'move-down',
        'move-left',
        'move-right',
        // Actions
        'plus',
        'minus',
        'x',
        'check',
        'check-circle',
        'check-square',
        'x-circle',
        'x-square',
        'trash',
        'trash-2',
        'edit',
        'edit-2',
        'edit-3',
        'pencil',
        'pencil-line',
        'save',
        'download',
        'upload',
        'copy',
        'clipboard',
        'clipboard-copy',
        'clipboard-check',
        'scissors',
        // Search & Filter
        'search',
        'filter',
        'filter-x',
        'sliders',
        'sliders-horizontal',
        // Settings & Tools
        'settings',
        'cog',
        'wrench',
        'hammer',
        'nut',
        // Security & Shield
        'shield',
        'shield-check',
        'shield-x',
        'shield-alert',
        'shield-off',
        'lock',
        'unlock',
        'key',
        'key-round',
        // Communication
        'mail',
        'mail-plus',
        'mail-minus',
        'mail-x',
        'mail-check',
        'message-square',
        'message-circle',
        'phone',
        'phone-call',
        'phone-incoming',
        'phone-outgoing',
        'phone-off',
        'bell',
        'bell-ring',
        'bell-off',
        // Media
        'image',
        'image-plus',
        'image-minus',
        'images',
        'camera',
        'video',
        'video-off',
        'music',
        'headphones',
        'play',
        'play-circle',
        'pause',
        'pause-circle',
        'stop-circle',
        'skip-back',
        'skip-forward',
        // Time & Calendar
        'clock',
        'clock-1',
        'clock-2',
        'clock-3',
        'clock-4',
        'clock-5',
        'clock-6',
        'clock-7',
        'clock-8',
        'clock-9',
        'clock-10',
        'clock-11',
        'clock-12',
        'calendar',
        'calendar-days',
        'calendar-check',
        'calendar-x',
        'calendar-plus',
        'calendar-minus',
        'calendar-clock',
        'timer',
        'hourglass',
        // Layout & UI
        'layout',
        'layout-dashboard',
        'layout-grid',
        'layout-list',
        'layout-template',
        'sidebar',
        'sidebar-open',
        'sidebar-close',
        'menu',
        'menu-square',
        'panel-left',
        'panel-right',
        'panel-top',
        'panel-bottom',
        'columns-2',
        'columns-3',
        'columns-4',
        'rows-2',
        'rows-3',
        'rows-4',
        // Status & Indicators
        'circle',
        'circle-dot',
        'square',
        'triangle',
        'triangle-alert',
        'alert-circle',
        'alert-triangle',
        'alert-octagon',
        'info',
        'help-circle',
        'check-circle-2',
        'x-circle',
        // Business & Finance
        'dollar-sign',
        'euro',
        'pound-sterling',
        'credit-card',
        'wallet',
        'receipt',
        'shopping-cart',
        'shopping-bag',
        'package',
        'box',
        'archive',
        // Location & Maps
        'map',
        'map-pin',
        'map-pin-plus',
        'map-pin-minus',
        'map-pin-x',
        'navigation',
        'compass',
        'globe',
        'globe-2',
        'home',
        'building',
        'building-2',
        // Network & Connectivity
        'wifi',
        'wifi-off',
        'signal',
        'signal-zero',
        'signal-low',
        'signal-medium',
        'signal-high',
        'link',
        'link-2',
        'unlink',
        'share',
        'share-2',
        'send',
        'send-horizontal',
        // Code & Development
        'code',
        'code-2',
        'brackets',
        'braces',
        'terminal',
        'command',
        'git-branch',
        'git-commit',
        'git-merge',
        'git-pull-request',
        'github',
        'gitlab',
        // Text & Typography
        'type',
        'bold',
        'italic',
        'underline',
        'strikethrough',
        'align-left',
        'align-center',
        'align-right',
        'align-justify',
        'list',
        'list-ordered',
        'list-checks',
        'heading-1',
        'heading-2',
        'heading-3',
        'heading-4',
        'heading-5',
        'heading-6',
        'quote',
        // Charts & Analytics
        'bar-chart',
        'bar-chart-2',
        'bar-chart-3',
        'line-chart',
        'pie-chart',
        'trending-up',
        'trending-down',
        'activity',
        // Workflow & Process
        'workflow',
        'git-branch',
        'git-merge',
        'git-pull-request',
        'git-commit',
        'git-fork',
        'git-compare',
        'git-merge',
        'git-pull-request-closed',
        'git-pull-request-draft',
        // Forms & Input
        'text-cursor',
        'text-cursor-input',
        'radio',
        'toggle-left',
        'toggle-right',
        // Tags & Labels
        'tag',
        'tags',
        'bookmark',
        'bookmark-plus',
        'bookmark-minus',
        'bookmark-x',
        'bookmark-check',
        // Notifications & Alerts
        'bell',
        'bell-ring',
        'bell-off',
        'alert-circle',
        'alert-triangle',
        'alert-octagon',
        'info',
        'help-circle',
        // Refresh & Sync
        'refresh-cw',
        'refresh-ccw',
        'rotate-cw',
        'rotate-ccw',
        'repeat',
        'repeat-1',
        // Visibility
        'eye',
        'eye-off',
        'eye-closed',
        // Power & Control
        'power',
        'power-off',
        'play',
        'pause',
        'skip-back',
        'skip-forward',
        'rewind',
        'fast-forward',
        // Storage & Files
        'hard-drive',
        'hard-drive-upload',
        'hard-drive-download',
        'server',
        'server-cog',
        'cloud',
        'cloud-upload',
        'cloud-download',
        'cloud-off',
        'cloud-rain',
        'cloud-snow',
        'cloud-lightning',
        // Organization
        'building',
        'building-2',
        'store',
        'warehouse',
        'factory',
        // Transportation
        'truck',
        'car',
        'bike',
        'plane',
        'ship',
        'train',
        'bus',
        // Health & Medical
        'heart',
        'heart-pulse',
        'activity',
        'cross',
        'crosshair',
        'stethoscope',
        // Education & Learning
        'book',
        'book-open',
        'book-marked',
        'graduation-cap',
        'school',
        'library',
        'award',
        'trophy',
        'medal',
        // Shopping & E-commerce
        'shopping-cart',
        'shopping-bag',
        'shopping-basket',
        'store',
        'receipt',
        'package',
        'box',
        'gift',
        // Social & Communication
        'message-square',
        'message-circle',
        'at-sign',
        'hash',
        // Miscellaneous
        'star',
        'star-off',
        'heart',
        'heart-off',
        'thumbs-up',
        'thumbs-down',
        'flag',
        'flag-off',
        'bookmark',
        'pin',
        'pin-off',
        'paperclip',
        'link',
        'unlink',
        'share',
        'share-2',
        'copy',
        'scissors',
        'undo',
        'redo',
        'rotate-cw',
        'rotate-ccw',
        'refresh-cw',
        'refresh-ccw',
        'maximize',
        'minimize',
        'maximize-2',
        'minimize-2',
        'expand',
        'shrink',
        'zoom-in',
        'zoom-out',
        'move',
        'grip',
        'grip-vertical',
        'grip-horizontal',
        'mouse-pointer',
        'mouse-pointer-click',
        'hand',
        'target',
        'crosshair',
        'focus',
        'sparkles',
        'zap',
        'flame',
        'droplet',
        'droplets',
        'sun',
        'moon',
        'sunrise',
        'sunset',
        'cloud',
        'cloud-sun',
        'cloud-moon',
        'cloud-rain',
        'cloud-snow',
        'cloud-lightning',
        'wind',
        'tornado',
        'rainbow',
        'umbrella',
        'umbrella-off',
        'tree-pine',
        'leaf',
        'flower',
        'flower-2',
        'mountain',
        'mountain-snow',
        'waves',
        'fish',
        'bug',
        'cat',
        'dog',
        'bird',
        'rabbit',
        'turtle',
    ].filter((icon, index, self) => self.indexOf(icon) === index) // Remove duplicates

    const filteredIcons = computed(() => {
        if (!searchQuery.value) {
            return popularIcons
        }
        return popularIcons.filter(icon =>
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

<style scoped>
    .icon-picker {
        width: 100%;
    }

    .icon-picker-card {
        width: 100%;
        max-height: 400px;
        overflow-y: auto;
    }

    .icon-grid {
        width: 100%;
    }

    /* Ensure icons are at 100% opacity */
    .icon-button :deep(.lucide-icon) {
        opacity: 1 !important;
    }

    .icon-button :deep(.v-btn__overlay) {
        opacity: 0;
    }
</style>
