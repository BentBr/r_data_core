import type { Plugin } from 'vite'

/**
 * Vite plugin to defer CSS loading
 * Uses the media="print" trick to load CSS asynchronously after page render
 */
export function deferCssPlugin(): Plugin {
    return {
        name: 'defer-css',
        enforce: 'post',
        transformIndexHtml(html) {
            // Find all CSS link tags and modify them to load asynchronously
            let modifiedHtml = html.replace(
                /<link([^>]*rel=["']stylesheet["'][^>]*)>/gi,
                (match, attributes) => {
                    // Skip if already has onload or media attribute (except if it's already print)
                    if (attributes.includes('onload=')) {
                        return match
                    }
                    // Extract href if present
                    const hrefMatch = attributes.match(/href=["']([^"']+)["']/i)
                    if (!hrefMatch) {
                        return match
                    }
                    const href = hrefMatch[1]
                    // Add media="print" and onload to switch to "all" after load
                    // Also add noscript fallback for browsers without JavaScript
                    const modifiedLink = `<link${attributes} media="print" onload="this.media='all'" />`
                    const noscriptFallback = `<noscript><link${attributes} /></noscript>`
                    return modifiedLink + noscriptFallback
                }
            )
            return modifiedHtml
        },
    }
}

