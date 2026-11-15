import adapter from '@sveltejs/adapter-static';
import tailwindcss from '@tailwindcss/vite';
import type { Config } from '@sveltejs/kit/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import wasm from 'vite-plugin-wasm';
import { defineConfig, loadEnv } from 'vite';

// Drop-in replacement for ../frontend: same base path, same aliases, same output layout.
//  - `pnpm build`         -> ./build (safe, never touches the current UI output)
//  - `pnpm build:dropin`  -> ../templates/html + ../static/v1 (REPLACES the current UI build)
const dropin = process.env.DROPIN === '1';

// Shared, presentation-free logic is used straight from the current UI so that upstream changes
// (API types, i18n texts, session/permission logic, helpers, passkey code) flow in automatically.
const OLD = '../frontend/src';

// SvelteKit 3 reads its configuration from here, `svelte.config.js` is not used anymore.
const kit: Config = {
	// CAUTION: `paths` must come before `adapter`
	paths: { base: '/auth/v1' },
	adapter: adapter(
		dropin
			? { pages: '../templates/html', assets: '../static/v1', precompress: true, strict: true }
			: { pages: 'build/html', assets: 'build/static', precompress: false, strict: true }
	),
	alias: {
		// removed in SvelteKit 3, the shadcn components still import it
		$lib: './src/lib',
		'@/*': './src/lib/*',
		// same alias names as ../frontend, so ported files keep their imports
		$lib5: './src/lib',
		$icons: './src/icons',
		// shared with ../frontend (logic only, no components)
		$api: `${OLD}/api`,
		$state: `${OLD}/global_state`,
		$utils: `${OLD}/utils`,
		$mfa: `${OLD}/mfa`,
		$i18n: `${OLD}/i18n`,
		$types: `${OLD}/types`,
		$workers: `${OLD}/workers`,
		$wasm: `${OLD}/wasm`
	},
	csp: {
		directives: {
			'default-src': ['none'],
			'connect-src': ['self'],
			'script-src': ['self', 'wasm-unsafe-eval'],
			'style-src': ['self', 'unsafe-inline'],
			// `https:` so the profile picture link of a federation provider (Google, Github, ...) can be
			// shown. Rauthy never fetches it, the browser does. Only `https`, scripts stay locked to hashes.
			'img-src': ['self', 'data:', 'https:'],
			'font-src': ['self']
		}
	}
};

export default defineConfig(({ mode }) => {
	const env = loadEnv(mode, process.cwd(), '');
	return {
		plugins: [
			tailwindcss(),
			wasm(),
			sveltekit({
				compilerOptions: {
					runes: ({ filename }: { filename: string }) =>
						filename.split(/[/\\]/).includes('node_modules') ? undefined : true
				},
				...kit
			})
		],
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
