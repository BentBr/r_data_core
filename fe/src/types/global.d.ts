/// <reference types="vite/client" />

// Global constants injected by Vite at build time
declare const __APP_VERSION__: string

// Allow importing any index.vue file even if it only exists virtually
declare module '*/index.vue' {
    import type { DefineComponent } from 'vue'
    const component: DefineComponent<Record<string, unknown>, Record<string, unknown>, unknown>
    export default component
}
