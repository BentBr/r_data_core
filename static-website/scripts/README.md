# Build Scripts

This directory contains Node.js scripts that run during the build process.

## Image Generation (`generate-images.js`)

Automatically generates responsive WebP images from source images in `assets/images/`.

**What it does:**
- Generates WebP versions at multiple sizes (400w, 800w, 1200w, 1600w)
- Creates a full-size WebP version
- Copies original PNG/JPG as fallback
- Outputs to `public/images/` for Vite to include in build

**Generated files:**
- `Slothlike-400w.webp` (12KB)
- `Slothlike-800w.webp` (40KB)
- `Slothlike-1200w.webp` (78KB)
- `Slothlike-1600w.webp` (116KB)
- `Slothlike.webp` (162KB)
- `Slothlike.png` (2.3MB - fallback)

**Usage in components:**
```vue
<picture>
    <source
        type="image/webp"
        srcset="
            /images/Slothlike-400w.webp 400w,
            /images/Slothlike-800w.webp 800w,
            /images/Slothlike-1200w.webp 1200w,
            /images/Slothlike-1600w.webp 1600w
        "
        sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 500px"
    />
    <img src="/images/Slothlike.png" alt="..." loading="lazy" />
</picture>
```

## Sitemap Generation (`generate-sitemap.js`)

Generates SEO-friendly sitemaps for all languages and pages.

**What it does:**
- Creates main `sitemap.xml` (index pointing to language-specific sitemaps)
- Generates `sitemap_en.xml` with all English pages
- Generates `sitemap_de.xml` with all German pages
- Creates `robots.txt` pointing to main sitemap
- Includes hreflang tags for proper language alternates

**Generated files:**
- `dist/sitemap.xml` - Main sitemap index
- `dist/sitemap_en.xml` - English pages sitemap
- `dist/sitemap_de.xml` - German pages sitemap
- `dist/robots.txt` - Search engine crawler instructions

**Sitemap features:**
- Language alternates (hreflang) for each page
- Automatic lastmod timestamps
- Priority and changefreq hints
- x-default for language-agnostic URLs

## Build Process

The build runs scripts in this order:

1. **`npm run generate:images`** - Generate responsive images
2. **`vue-tsc`** - TypeScript compilation check
3. **`vite build`** - Build the Vue app
4. **`npm run generate:sitemap`** - Generate sitemaps

To run individually:
```bash
npm run generate:images
npm run generate:sitemap
```

## Configuration

### Base URL
Set `VITE_BASE_URL` in `.env` or `compose.yaml` to configure the base URL for sitemaps:
```
VITE_BASE_URL=https://rdatacore.eu
```

### Image Sizes
Edit `sizes` array in `generate-images.js`:
```javascript
const sizes = [400, 800, 1200, 1600]
```

### Pages
Edit `pages` array in `generate-sitemap.js`:
```javascript
const pages = ['/', '/about', '/pricing']
```

### Languages
Edit `languages` array in `generate-sitemap.js`:
```javascript
const languages = ['en', 'de']
```

