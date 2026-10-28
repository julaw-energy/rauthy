import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import wasm from 'vite-plugin-wasm';
import { defineConfig, loadEnv } from 'vite';

export default defineConfig(({ mode }) => {
	const env = loadEnv(mode, process.cwd(), '');
	return {
		plugins: [tailwindcss(), wasm(), sveltekit()],
		build: {
			target: 'esnext',
			// Never inline a script as a `data:` URI (the default for files under 4 KB): the CSP only allows
			// `script-src 'self'`, so the browser would refuse the theme script loaded from <head>.
			assetsInlineLimit: (file) => (file.endsWith('.js') ? false : undefined)
		},
		worker: { format: 'es', plugins: () => [wasm()] },
		// logic modules are shared with ../frontend: make sure there is exactly ONE copy of these
		resolve: { dedupe: ['svelte', 'base64-arraybuffer'] },
		server: {
			host: env.VITE_DEV_HOST || '127.0.0.1',
			port: env.VITE_DEV_PORT ? Number(env.VITE_DEV_PORT) : 5174,
			allowedHosts: env.VITE_DEV_ALLOWED_HOSTS ? env.VITE_DEV_ALLOWED_HOSTS.split(',') : undefined,
			strictPort: true,
			fs: { allow: ['..'] },
			hmr: env.VITE_HMR_HOST ? { protocol: 'wss', host: env.VITE_HMR_HOST, clientPort: 443 } : undefined
			// No `proxy`: Traefik routes API paths straight to the Rauthy backend (see local-dev/).
		}
	};
});
