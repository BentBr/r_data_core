import { ref, computed, watch, defineComponent } from 'vue'
import LucideIcon from '../LucideIcon/index.vue'
import SmartIcon from '../SmartIcon/index.vue'

export default defineComponent({
    name: 'IconPicker',
    components: {
        LucideIcon,
        SmartIcon,
    },
    props: {
        modelValue: {
            type: String,
            default: '',
        },
        label: {
            type: String,
            default: 'Select Icon',
        },
        showPicker: {
            type: Boolean,
            default: true,
        },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const searchQuery = ref('')
        const selectedIcon = ref(props.modelValue ?? '')

        const popularIcons = [
            'file', 'file-text', 'file-edit', 'file-plus', 'file-minus', 'file-x', 'file-check', 'file-search', 'file-code', 'file-image', 'file-video', 'file-audio', 'file-type',
            'folder', 'folder-open', 'folder-plus', 'folder-minus', 'folder-x', 'folder-check', 'folder-search', 'folder-tree', 'folder-symlink',
            'database', 'table', 'table-2', 'columns', 'rows', 'layout-grid', 'layout-list',
            'user', 'user-plus', 'user-minus', 'user-x', 'user-check', 'users', 'user-circle', 'user-square', 'user-cog', 'user-search',
            'arrow-up', 'arrow-down', 'arrow-left', 'arrow-right', 'arrow-up-down', 'arrow-left-right', 'chevron-up', 'chevron-down', 'chevron-left', 'chevron-right', 'chevrons-up', 'chevrons-down', 'chevrons-left', 'chevrons-right', 'move', 'move-up', 'move-down', 'move-left', 'move-right',
            'plus', 'minus', 'x', 'check', 'check-circle', 'check-square', 'x-circle', 'x-square', 'trash', 'trash-2', 'edit', 'edit-2', 'edit-3', 'pencil', 'pencil-line', 'save', 'download', 'upload', 'copy', 'clipboard', 'clipboard-copy', 'clipboard-check', 'scissors',
            'search', 'filter', 'filter-x', 'sliders', 'sliders-horizontal',
            'settings', 'cog', 'wrench', 'hammer', 'nut',
            'shield', 'shield-check', 'shield-x', 'shield-alert', 'shield-off', 'lock', 'unlock', 'key', 'key-round',
            'mail', 'mail-plus', 'mail-minus', 'mail-x', 'mail-check', 'message-square', 'message-circle', 'phone', 'phone-call', 'phone-incoming', 'phone-outgoing', 'phone-off', 'bell', 'bell-ring', 'bell-off',
            'image', 'image-plus', 'image-minus', 'images', 'camera', 'video', 'video-off', 'music', 'headphones', 'play', 'play-circle', 'pause', 'pause-circle', 'stop-circle', 'skip-back', 'skip-forward',
            'clock', 'clock-1', 'clock-2', 'clock-3', 'clock-4', 'clock-5', 'clock-6', 'clock-7', 'clock-8', 'clock-9', 'clock-10', 'clock-11', 'clock-12', 'calendar', 'calendar-days', 'calendar-check', 'calendar-x', 'calendar-plus', 'calendar-minus', 'calendar-clock', 'timer', 'hourglass',
            'layout', 'layout-dashboard', 'layout-grid', 'layout-list', 'layout-template', 'sidebar', 'sidebar-open', 'sidebar-close', 'menu', 'menu-square', 'panel-left', 'panel-right', 'panel-top', 'panel-bottom', 'columns-2', 'columns-3', 'columns-4', 'rows-2', 'rows-3', 'rows-4',
            'circle', 'circle-dot', 'square', 'triangle', 'triangle-alert', 'alert-circle', 'alert-triangle', 'alert-octagon', 'info', 'help-circle', 'check-circle-2', 'x-circle',
            'dollar-sign', 'euro', 'pound-sterling', 'credit-card', 'wallet', 'receipt', 'shopping-cart', 'shopping-bag', 'package', 'box', 'archive',
            'map', 'map-pin', 'map-pin-plus', 'map-pin-minus', 'map-pin-x', 'navigation', 'compass', 'globe', 'globe-2', 'home', 'building', 'building-2',
            'wifi', 'wifi-off', 'signal', 'signal-zero', 'signal-low', 'signal-medium', 'signal-high', 'link', 'link-2', 'unlink', 'share', 'share-2', 'send', 'send-horizontal',
            'code', 'code-2', 'brackets', 'braces', 'terminal', 'command', 'git-branch', 'git-commit', 'git-merge', 'git-pull-request', 'github', 'gitlab',
            'type', 'bold', 'italic', 'underline', 'strikethrough', 'align-left', 'align-center', 'align-right', 'align-justify', 'list', 'list-ordered', 'list-checks', 'heading-1', 'heading-2', 'heading-3', 'heading-4', 'heading-5', 'heading-6', 'quote',
            'bar-chart', 'bar-chart-2', 'bar-chart-3', 'line-chart', 'pie-chart', 'trending-up', 'trending-down', 'activity',
            'workflow', 'git-branch', 'git-merge', 'git-pull-request', 'git-commit', 'git-fork', 'git-compare', 'git-merge', 'git-pull-request-closed', 'git-pull-request-draft',
            'text-cursor', 'text-cursor-input', 'radio', 'toggle-left', 'toggle-right',
            'tag', 'tags', 'bookmark', 'bookmark-plus', 'bookmark-minus', 'bookmark-x', 'bookmark-check',
            'bell', 'bell-ring', 'bell-off', 'alert-circle', 'alert-triangle', 'alert-octagon', 'info', 'help-circle',
            'refresh-cw', 'refresh-ccw', 'rotate-cw', 'rotate-ccw', 'repeat', 'repeat-1',
            'eye', 'eye-off', 'eye-closed',
            'power', 'power-off', 'play', 'pause', 'skip-back', 'skip-forward', 'rewind', 'fast-forward',
            'hard-drive', 'hard-drive-upload', 'hard-drive-download', 'server', 'server-cog', 'cloud', 'cloud-upload', 'cloud-download', 'cloud-off', 'cloud-rain', 'cloud-snow', 'cloud-lightning',
            'building', 'building-2', 'store', 'warehouse', 'factory',
            'truck', 'car', 'bike', 'plane', 'ship', 'train', 'bus',
            'heart', 'heart-pulse', 'activity', 'cross', 'crosshair', 'stethoscope',
            'book', 'book-open', 'book-marked', 'graduation-cap', 'school', 'library', 'award', 'trophy', 'medal',
            'shopping-cart', 'shopping-bag', 'shopping-basket', 'store', 'receipt', 'package', 'box', 'gift',
            'message-square', 'message-circle', 'at-sign', 'hash',
            'star', 'star-off', 'heart', 'heart-off', 'thumbs-up', 'thumbs-down', 'flag', 'flag-off', 'bookmark', 'pin', 'pin-off', 'paperclip', 'link', 'unlink', 'share', 'share-2', 'copy', 'scissors', 'undo', 'redo', 'rotate-cw', 'rotate-ccw', 'refresh-cw', 'refresh-ccw', 'maximize', 'minimize', 'maximize-2', 'minimize-2', 'expand', 'shrink', 'zoom-in', 'zoom-out', 'move', 'grip', 'grip-vertical', 'grip-horizontal', 'mouse-pointer', 'mouse-pointer-click', 'hand', 'target', 'crosshair', 'focus', 'sparkles', 'zap', 'flame', 'droplet', 'droplets', 'sun', 'moon', 'sunrise', 'sunset', 'cloud', 'cloud-sun', 'cloud-moon', 'cloud-rain', 'cloud-snow', 'cloud-lightning', 'wind', 'tornado', 'rainbow', 'umbrella', 'umbrella-off', 'tree-pine', 'leaf', 'flower', 'flower-2', 'mountain', 'mountain-snow', 'waves', 'fish', 'bug', 'cat', 'dog', 'bird', 'rabbit', 'turtle',
        ].filter((icon, index, self) => self.indexOf(icon) === index)

        const filteredIcons = computed(() => {
            if (!searchQuery.value) return popularIcons
            return popularIcons.filter(icon => icon.toLowerCase().includes(searchQuery.value.toLowerCase()))
        })

        const selectIcon = (icon: string) => {
            selectedIcon.value = icon
            emit('update:modelValue', icon)
        }

        watch(() => props.modelValue, newValue => { selectedIcon.value = newValue ?? '' })

        return {
            searchQuery, selectedIcon, filteredIcons, selectIcon,
        }
    },
})
