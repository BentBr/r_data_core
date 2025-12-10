import sharp from 'sharp'
import { promises as fs } from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const sourceDir = path.join(__dirname, '../assets/images')
const outputDir = path.join(__dirname, '../public/images')

async function ensureDir(dir) {
    try {
        await fs.access(dir)
    } catch {
        await fs.mkdir(dir, { recursive: true })
    }
}

async function generateFavicons() {
    const sourcePath = path.join(sourceDir, 'sloth_favicon_1024.png')
    
    try {
        // Check if source file exists
        await fs.access(sourcePath)
    } catch {
        console.error('❌ Source favicon not found:', sourcePath)
        process.exit(1)
    }

    console.log('Processing favicon sloth_favicon_1024.png...')

    // Generate ICO-compatible sizes (16, 32, 48)
    for (const size of [16, 32, 48]) {
        const outputPath = path.join(outputDir, `favicon-${size}x${size}.png`)
        await sharp(sourcePath).resize(size, size).png().toFile(outputPath)
        console.log(`  Generated favicon-${size}x${size}.png`)
    }

    // Generate Apple Touch Icon (180x180)
    const appleTouchIcon = path.join(outputDir, 'apple-touch-icon.png')
    await sharp(sourcePath).resize(180, 180).png().toFile(appleTouchIcon)
    console.log(`  Generated apple-touch-icon.png (180x180)`)

    // Generate PWA icons (192x192, 512x512)
    for (const size of [192, 512]) {
        const outputPath = path.join(outputDir, `icon-${size}x${size}.png`)
        await sharp(sourcePath).resize(size, size).png().toFile(outputPath)
        console.log(`  Generated icon-${size}x${size}.png`)
    }

    // Generate standard favicon (32x32)
    const faviconPath = path.join(outputDir, 'favicon.png')
    await sharp(sourcePath).resize(32, 32).png().toFile(faviconPath)
    console.log(`  Generated favicon.png (32x32)`)

    console.log('✅ All favicons generated successfully!')
}

async function main() {
    try {
        await ensureDir(outputDir)
        await generateFavicons()
    } catch (error) {
        console.error('❌ Error generating favicons:', error)
        process.exit(1)
    }
}

main()

