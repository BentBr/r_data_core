import { computed, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useHead } from '@vueuse/head'
import { env, getBaseUrl } from '@/env-check'
import { useTranslations } from './useTranslations'

type SeoOptions = {
    title?: string | (() => string)
    description?: string | (() => string)
    keywords?: string[]
    siteName?: string
    locale?: string
}

function resolveOption(option: string | (() => string) | undefined): string | undefined {
    if (typeof option === 'function') {
        return option()
    }
    return option
}

export function useSEO(options: SeoOptions) {
    const route = useRoute()
    const { currentLanguage } = useTranslations()

    // Create computed values for reactive head management
    const headData = computed(() => {
        const isNoIndex = route.meta.noIndex === true
        const siteName = env.siteName ?? 'RDataCore'
        const baseUrl = getBaseUrl()

        const resolvedTitle = resolveOption(options.title)
        const resolvedDescription = resolveOption(options.description)
        const baseTitle = resolvedTitle ?? siteName
        const fullTitle = `${baseTitle} · ${siteName}`
        const description = resolvedDescription ?? siteName
        const keywords = options.keywords?.join(', ') ?? 'data, rdatacore'
        // Derive locale from route path for SSR compatibility (router guard may not have run yet)
        const routeLang = route.path.match(/^\/(en|de)/)?.[1]
        const locale = routeLang ?? currentLanguage.value

        const canonicalUrl = `${baseUrl}${route.fullPath}`

        // Build language-specific URLs for hreflang
        const pathWithoutLang = route.path.replace(/^\/(en|de)/, '') || '/'
        const altEn = `${baseUrl}/en${pathWithoutLang === '/' ? '' : pathWithoutLang}`
        const altDe = `${baseUrl}/de${pathWithoutLang === '/' ? '' : pathWithoutLang}`
        const altDefault = `${baseUrl}${pathWithoutLang}`

        return {
            title: fullTitle,
            meta: [
                { name: 'description', content: description },
                { name: 'keywords', content: keywords },
                { name: 'robots', content: isNoIndex ? 'noindex,nofollow' : 'index,follow' },
                // Open Graph
                { property: 'og:title', content: fullTitle },
                { property: 'og:description', content: description },
                { property: 'og:url', content: canonicalUrl },
                { property: 'og:site_name', content: siteName },
                { property: 'og:locale', content: locale === 'de' ? 'de_DE' : 'en_GB' },
                { property: 'og:type', content: 'website' },
                { property: 'og:image', content: `${baseUrl}/images/Slothlike-og.jpg` },
                { property: 'og:image:width', content: '1200' },
                { property: 'og:image:height', content: '630' },
                { property: 'og:image:alt', content: siteName },
                // Twitter
                { property: 'twitter:card', content: 'summary_large_image' },
                { property: 'twitter:title', content: fullTitle },
                { property: 'twitter:description', content: description },
                { property: 'twitter:image', content: `${baseUrl}/images/Slothlike-twitter.jpg` },
            ],
            link: [
                { rel: 'canonical', href: canonicalUrl },
                { rel: 'alternate', href: altEn, hreflang: 'en' },
                { rel: 'alternate', href: altDe, hreflang: 'de' },
                { rel: 'alternate', href: altDefault, hreflang: 'x-default' },
            ],
            htmlAttrs: {
                lang: locale,
            },
        }
    })

    // Use @vueuse/head for SSG-compatible head management
    useHead(headData)

    // Watch for language changes to update head
    watch(currentLanguage, () => {
        // Head is automatically updated via computed
    })
}
