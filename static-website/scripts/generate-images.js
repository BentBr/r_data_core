import sharp from 'sharp'
import { promises as fs } from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const sizes = [400, 800, 1200, 1600]
const inputDir = path.join(__dirname, '../assets/images')
const outputDir = path.join(__dirname, '../public/images')

async function generateResponsiveImages() {
    try {
        // Ensure output directory exists
        await fs.mkdir(outputDir, { recursive: true })

        const files = await fs.readdir(inputDir)
        const imageFiles = files.filter((file) =>
            /\.(jpg|jpeg|png)$/i.test(file)
        )

        for (const file of imageFiles) {
            const inputPath = path.join(inputDir, file)
            const baseName = path.basename(file, path.extname(file))

            console.log(`Processing ${file}...`)

            // Generate WebP versions at different sizes
            for (const size of sizes) {
                const outputPath = path.join(
                    outputDir,
                    `${baseName}-${size}w.webp`
                )
                await sharp(inputPath)
                    .resize(size, null, {
                        withoutEnlargement: true,
                        fit: 'inside',
                    })
                    .webp({ quality: 85 })
                    .toFile(outputPath)

                console.log(`  Generated ${baseName}-${size}w.webp`)
            }

            // Generate original WebP (full size)
            const fullSizeOutputPath = path.join(
                outputDir,
                `${baseName}.webp`
            )
            await sharp(inputPath)
                .webp({ quality: 90 })
                .toFile(fullSizeOutputPath)

            console.log(`  Generated ${baseName}.webp (full size)`)

            // Also copy PNG fallback
            const fallbackPath = path.join(outputDir, file)
            await fs.copyFile(inputPath, fallbackPath)
            console.log(`  Copied ${file} as fallback`)
        }

        console.log('✅ All images processed successfully!')
    } catch (error) {
        console.error('Error processing images:', error)
        process.exit(1)
    }
}

generateResponsiveImages()

