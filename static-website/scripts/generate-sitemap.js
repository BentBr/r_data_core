import { promises as fs } from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const baseUrl = process.env.VITE_BASE_URL || 'https://rdatacore.eu'
const outputDir = path.join(__dirname, '../dist')

// Only include indexable pages (excluding imprint, privacy)
const pages = ['/', '/about', '/pricing']
const languages = ['en', 'de']

function generateSitemapIndex() {
    return `<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <sitemap>
    <loc>${baseUrl}/sitemap_en.xml</loc>
    <lastmod>${new Date().toISOString()}</lastmod>
  </sitemap>
  <sitemap>
    <loc>${baseUrl}/sitemap_de.xml</loc>
    <lastmod>${new Date().toISOString()}</lastmod>
  </sitemap>
</sitemapindex>`
}

function generateLanguageSitemap(lang) {
    const urls = pages.map((page) => {
        const loc = page === '/' ? `/${lang}` : `/${lang}${page}`
        return `  <url>
    <loc>${baseUrl}${loc}</loc>
    <lastmod>${new Date().toISOString()}</lastmod>
    <changefreq>weekly</changefreq>
    <priority>${page === '/' ? '1.0' : '0.8'}</priority>
    <xhtml:link rel="alternate" hreflang="en" href="${baseUrl}/en${page === '/' ? '' : page}"/>
    <xhtml:link rel="alternate" hreflang="de" href="${baseUrl}/de${page === '/' ? '' : page}"/>
    <xhtml:link rel="alternate" hreflang="x-default" href="${baseUrl}${page}"/>
  </url>`
    })

    return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"
        xmlns:xhtml="http://www.w3.org/1999/xhtml">
${urls.join('\n')}
</urlset>`
}

function generateRobotsTxt() {
    return `# https://www.robotstxt.org/robotstxt.html
User-agent: *
Allow: /

Sitemap: ${baseUrl}/sitemap.xml
`
}

async function generateSitemaps() {
    try {
        // Generate main sitemap index
        const sitemapIndex = generateSitemapIndex()
        await fs.writeFile(
            path.join(outputDir, 'sitemap.xml'),
            sitemapIndex
        )
        console.log('✅ Generated sitemap.xml')

        // Generate language-specific sitemaps
        for (const lang of languages) {
            const sitemap = generateLanguageSitemap(lang)
            await fs.writeFile(
                path.join(outputDir, `sitemap_${lang}.xml`),
                sitemap
            )
            console.log(`✅ Generated sitemap_${lang}.xml`)
        }

        // Generate robots.txt
        const robotsTxt = generateRobotsTxt()
        await fs.writeFile(path.join(outputDir, 'robots.txt'), robotsTxt)
        console.log('✅ Generated robots.txt')

        console.log('✅ All sitemaps and robots.txt generated successfully!')
    } catch (error) {
        console.error('Error generating sitemaps:', error)
        process.exit(1)
    }
}

generateSitemaps()

