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

    plugins: [
        {
            name: 'handle-shorthand-urls',
            configureServer(server) {
                server.middlewares.use((req, res, next) => {
                    const url = (req as any).url || '';

                    // Redirect /benchmarks to /jokkerin-ventti/benchmarks/
                    if (url === '/benchmarks' || url === '/benchmarks/') {
                        res.writeHead(301, { Location: '/jokkerin-ventti/benchmarks/' });
                        res.end();
                        return;
                    }

                    // Redirect /jokkerin-ventti (no slash) to /jokkerin-ventti/
                    if (url === '/jokkerin-ventti') {
                        res.writeHead(301, { Location: '/jokkerin-ventti/' });
                        res.end();
                        return;
                    }

                    // Handle /jokkerin-ventti/benchmarks (missing trailing slash)
                    if (url === '/jokkerin-ventti/benchmarks') {
                        res.writeHead(301, { Location: '/jokkerin-ventti/benchmarks/' });
                        res.end();
                        return;
                    }

                    next();
                });
            }
        }
    ]
})
