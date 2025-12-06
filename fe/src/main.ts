import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import { checkEnvironmentVariables, env } from './env-check'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import type { IconAliases } from 'vuetify'
import SmartIcon from '@/components/common/SmartIcon.vue'

// Vuetify styles
import 'vuetify/styles'

// Design system theme configuration
import { vuetifyTheme } from './design-system'

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
    expand: 'chevron-down',
    menu: 'menu',
    subgroup: 'chevron-down',
    dropdown: 'chevron-down',
    menuRight: 'chevron-right',
    menuDown: 'chevron-down',
    menuLeft: 'chevron-left',
    menuUp: 'chevron-up',
    'mdi-menu-right': 'chevron-right',
    'mdi-menu-down': 'chevron-down',
    'mdi-menu-left': 'chevron-left',
    'mdi-menu-up': 'chevron-up',
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
    components,
    directives,
    theme: vuetifyTheme,
    icons: {
        defaultSet: 'smart',
        aliases: iconAliases,
        sets: {
            smart: {
                component: SmartIcon as unknown as any,
            },
        },
    },
})

// Check environment variables in development
if (env.isDevelopment) {
    checkEnvironmentVariables()
}

const app = createApp(App)

app.use(createPinia())
app.use(router)
app.use(vuetify)

app.mount('#app')
