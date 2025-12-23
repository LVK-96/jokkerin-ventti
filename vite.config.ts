import { defineConfig } from 'vite'

export default defineConfig({
    // Base path for GitHub Pages (repository name)
    // Update this if your repo is named differently
    base: '/jokkerin-ventti/',

    server: {
        port: 5173,
        strictPort: true,
        fs: {
            allow: ['.']
        }
    },

    appType: 'mpa',

    build: {
        outDir: 'dist',
        rollupOptions: {
            input: {
                main: './index.html',
                benchmarks: './benchmarks/index.html',
            },
        },
    },
})
