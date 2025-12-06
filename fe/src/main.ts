import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import { checkEnvironmentVariables, env } from './env-check'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'

// Vuetify styles
import 'vuetify/styles'

// Design system theme configuration
import { vuetifyTheme } from './design-system'

const vuetify = createVuetify({
    components,
    directives,
    theme: vuetifyTheme,
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
