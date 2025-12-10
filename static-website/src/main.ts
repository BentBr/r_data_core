import { createApp } from 'vue'
import { createVuetify } from 'vuetify'
// Import only the components we use for tree-shaking
import {
    VApp,
    VBtn,
    VCard,
    VCardActions,
    VCardText,
    VCardTitle,
    VCol,
    VContainer,
    VDialog,
    VList,
    VListItem,
    VListItemTitle,
    VMenu,
    VRow,
} from 'vuetify/components'
import router from './router'
import App from './App.vue'
import { vuetifyDefaults, vuetifyTheme } from './design-system'
import SmartIcon from './components/common/SmartIcon.vue'
import type { IconAliases } from 'vuetify'
import { checkEnvironmentVariables } from './env-check'

import 'vuetify/styles'

const iconAliases: Partial<IconAliases> = {
    collapse: 'chevron-up',
    complete: 'check',
    cancel: 'x',
    close: 'x',
    delete: 'trash-2',
    clear: 'x-circle',
    success: 'check-circle',
    info: 'info',
    warning: 'alert-triangle',
    error: 'alert-octagon',
    prev: 'chevron-left',
    next: 'chevron-right',
    first: 'chevrons-left',
    last: 'chevrons-right',
    delimiter: 'circle',
    sort: 'arrow-up-down',
    sortAsc: 'arrow-up',
    sortDesc: 'arrow-down',
    expand: 'chevron-down',
    menu: 'menu',
    subgroup: 'chevron-down',
    dropdown: 'chevron-down',
    menuRight: 'chevron-right',
    menuDown: 'chevron-down',
    menuLeft: 'chevron-left',
    menuUp: 'chevron-up',
    radioOn: 'dot',
    radioOff: 'circle',
    edit: 'pencil',
    ratingEmpty: 'star',
    ratingFull: 'star',
    ratingHalf: 'star-half',
    checkboxOn: 'check-square',
    checkboxOff: 'square',
    checkboxIndeterminate: 'minus-square',
}

const vuetify = createVuetify({
    components: {
        VApp,
        VBtn,
        VCard,
        VCardActions,
        VCardText,
        VCardTitle,
        VCol,
        VContainer,
        VDialog,
        VList,
        VListItem,
        VListItemTitle,
        VMenu,
        VRow,
    },
    theme: vuetifyTheme,
    defaults: vuetifyDefaults,
    icons: {
        defaultSet: 'smart',
        aliases: iconAliases,
        sets: {
            smart: {
                // Type assertion needed due to Vuetify IconComponent type mismatch
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                component: SmartIcon as unknown as any,
            },
        },
    },
})

// Log environment variables in dev for sanity
if (import.meta.env.DEV) {
    checkEnvironmentVariables()
}

const app = createApp(App)
app.use(router)
app.use(vuetify)
app.mount('#app')
