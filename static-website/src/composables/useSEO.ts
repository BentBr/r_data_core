import { onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { env } from '@/env-check'
import { useTranslations } from './useTranslations'

type SeoOptions = {
    title: string
    description: string
    keywords?: string[]
    locale: string
}

function setMetaTag(name: string, content: string) {
    let tag = document.querySelector<HTMLMetaElement>(`meta[name="${name}"]`)
    if (!tag) {
        tag = document.createElement('meta')
        tag.setAttribute('name', name)
        document.head.appendChild(tag)
    }
    tag.setAttribute('content', content)
}

function setPropertyTag(property: string, content: string) {
    let tag = document.querySelector<HTMLMetaElement>(`meta[property="${property}"]`)
    if (!tag) {
        tag = document.createElement('meta')
        tag.setAttribute('property', property)
        document.head.appendChild(tag)
    }
    tag.setAttribute('content', content)
}

function setLinkTag(rel: string, href: string, hreflang?: string) {
    let tag = document.querySelector<HTMLLinkElement>(
        `link[rel="${rel}"][${hreflang ? 'hreflang' : 'href'}="${hreflang ?? href}"]`
    )
    if (!tag) {
        tag = document.createElement('link')
        tag.setAttribute('rel', rel)
        if (hreflang) {
            tag.setAttribute('hreflang', hreflang)
        }
        document.head.appendChild(tag)
    }
    tag.setAttribute('href', href)
    if (hreflang) {
        tag.setAttribute('hreflang', hreflang)
    }
}

export function useSEO(options: SeoOptions) {
    const route = useRoute()
    const { currentLanguage } = useTranslations()

    const applySeo = () => {
        const siteName = env.siteName || 'RDataCore'
        const baseTitle = options.title || siteName
        const fullTitle = `${baseTitle} · ${siteName}`
        const description = options.description || siteName
        const keywords = options.keywords?.join(', ') ?? 'data, rdatacore'
        const locale = options.locale || currentLanguage.value

        document.title = fullTitle
        setMetaTag('description', description)
        setMetaTag('keywords', keywords)

        const canonicalUrl = `${env.baseUrl}${route.fullPath}`
        setLinkTag('canonical', canonicalUrl)

        const altEn = `${env.baseUrl}/`
        const altDe = `${env.baseUrl}/`
        setLinkTag('alternate', altEn, 'en')
        setLinkTag('alternate', altDe, 'de')
        setLinkTag('alternate', canonicalUrl, 'x-default')

        setPropertyTag('og:title', fullTitle)
        setPropertyTag('og:description', description)
        setPropertyTag('og:url', canonicalUrl)
        setPropertyTag('og:site_name', siteName)
        setPropertyTag('og:locale', locale === 'de' ? 'de_DE' : 'en_US')
        setPropertyTag('twitter:card', 'summary')
        setPropertyTag('twitter:title', fullTitle)
        setPropertyTag('twitter:description', description)
    }

    onMounted(applySeo)
    watch(
        () => [route.fullPath, currentLanguage.value],
        () => applySeo()
    )
}
