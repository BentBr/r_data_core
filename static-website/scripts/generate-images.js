import sharp from 'sharp'
import { promises as fs } from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const sourceDir = path.join(__dirname, '../assets/images')
const outputDir = path.join(__dirname, '../public/images')

// Responsive image widths
const widths = [400, 800, 1200, 1600]

async function ensureDir(dir) {
    try {
        await fs.access(dir)
    } catch {
        await fs.mkdir(dir, { recursive: true })
    }
}

async function processFavicon(filename) {
    const sourcePath = path.join(sourceDir, filename)
    console.log(`Processing favicon ${filename}...`)

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
}

async function processOgImage(filename) {
    const sourcePath = path.join(sourceDir, filename)
    const baseName = path.basename(filename, path.extname(filename))
    console.log(`Processing OG image ${filename}...`)

    // Open Graph recommended size: 1200x630 (1.91:1 ratio)
    const ogOutputPath = path.join(outputDir, `${baseName}-og.jpg`)
    await sharp(sourcePath)
        .resize(1200, 630, {
            fit: 'cover',
            position: 'center',
        })
        .jpeg({ quality: 90 })
        .toFile(ogOutputPath)
    console.log(`  Generated ${baseName}-og.jpg (1200x630)`)

    // Twitter Card (1200x600)
    const twitterOutputPath = path.join(outputDir, `${baseName}-twitter.jpg`)
    await sharp(sourcePath)
        .resize(1200, 600, {
            fit: 'cover',
            position: 'center',
        })
        .jpeg({ quality: 90 })
        .toFile(twitterOutputPath)
    console.log(`  Generated ${baseName}-twitter.jpg (1200x600)`)
}

async function processImage(filename) {
    const sourcePath = path.join(sourceDir, filename)
    const baseName = path.basename(filename, path.extname(filename))

    console.log(`Processing ${filename}...`)

    // Generate responsive WebP versions
    for (const width of widths) {
        const outputPath = path.join(outputDir, `${baseName}-${width}w.webp`)
        await sharp(sourcePath).resize(width, null).webp({ quality: 85 }).toFile(outputPath)
        console.log(`  Generated ${baseName}-${width}w.webp`)
    }

    // Generate full-size WebP
    const fullSizeWebp = path.join(outputDir, `${baseName}.webp`)
    await sharp(sourcePath).webp({ quality: 90 }).toFile(fullSizeWebp)
    console.log(`  Generated ${baseName}.webp (full size)`)

    // Copy original PNG as fallback
    const pngOutput = path.join(outputDir, filename)
    await fs.copyFile(sourcePath, pngOutput)
    console.log(`  Copied ${filename} as fallback`)
}

async function main() {
    try {
        await ensureDir(outputDir)

        // Get all PNG files from source directory
        const files = await fs.readdir(sourceDir)
        const pngFiles = files.filter((file) => file.endsWith('.png'))

        for (const file of pngFiles) {
            // Process favicon separately
            if (file === 'sloth_favicon_1024.png') {
                await processFavicon(file)
                continue
            }

            // Process Slothlike.png for OG images
            if (file === 'Slothlike.png') {
                await processOgImage(file)
            }

            // Process all images normally (responsive versions)
            await processImage(file)
        }

        console.log('✅ All images processed successfully!')
    } catch (error) {
        console.error('❌ Error processing images:', error)
        process.exit(1)
    }
}

main()
