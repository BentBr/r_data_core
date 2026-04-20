import { existsSync, readFileSync } from 'node:fs';
import { join, basename, dirname, resolve } from 'node:path';
import type { Plugin } from 'vite';

/**
 * A Vite plugin that provides virtual index.vue files for component directories
 * that follow the separate-file structure (HTML, TS, SCSS).
 */
export function VirtualVueSFCPlugin(): Plugin {
    return {
        name: 'virtual-vue-sfc',
        enforce: 'pre',
        resolveId(id, importer) {
            // Only handle index.vue imports
            if (id.endsWith('index.vue')) {
                // Determine absolute path
                let fullPath = id;
                if (importer && id.startsWith('.')) {
                    fullPath = resolve(dirname(importer), id);
                } else if (id.startsWith('/src/')) {
                    // This could be an absolute-style path from Vite
                    fullPath = resolve(process.cwd(), id.substring(1));
                }
                
                // If it doesn't exist on disk, check if it's a separate-file component
                if (fullPath.endsWith('index.vue') && !existsSync(fullPath)) {
                    const dir = dirname(fullPath);
                    const name = basename(dir);
                    if (existsSync(join(dir, `${name}.ts`)) && existsSync(join(dir, `${name}.html`))) {
                        return fullPath; // Claim this ID
                    }
                }
            }
            return null;
        },
        async load(id) {
            // The id here will be the absolute path after resolution
            if (id.endsWith('index.vue') && id.includes('/src/') && !existsSync(id)) {
                const dir = dirname(id);
                const name = basename(dir);
                
                // Check if the required files exist in the same directory
                const tsPath = join(dir, `${name}.ts`);
                const htmlPath = join(dir, `${name}.html`);
                const scssPath = join(dir, `${name}.scss`);

                if (existsSync(tsPath) && existsSync(htmlPath)) {
                    const hasScss = existsSync(scssPath);
                    // App/index.vue is usually NOT scoped, others are.
                    const isApp = name === 'App';
                    const scoped = isApp ? '' : 'scoped';
                    
                    // Inline the template so auto-registration plugins (like unplugin-vue-components)
                    // can scan it for components.
                    const htmlContent = readFileSync(htmlPath, 'utf-8');
                    
                    const code = `<template>
${htmlContent}
</template>
<script lang="ts" src="./${name}.ts"></script>
${hasScss ? `<style ${scoped} lang="scss" src="./${name}.scss"></style>` : ''}
`;
                    return {
                        code,
                        map: null,
                    };
                }
            }
            return null;
        },
    };
}
