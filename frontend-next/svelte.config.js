import adapter from '@sveltejs/adapter-static';

// Drop-in replacement for ../frontend: same base path, same aliases, same output layout.
//  - `pnpm build`         -> ./build (safe, never touches the current UI output)
//  - `pnpm build:dropin`  -> ../templates/html + ../static/v1 (REPLACES the current UI build)
const dropin = process.env.DROPIN === '1';

// Shared, presentation-free logic is used straight from the current UI so that upstream changes
// (API types, i18n texts, session/permission logic, helpers, passkey code) flow in automatically.
const OLD = '../frontend/src';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	compilerOptions: {
		runes: ({ filename }) => (filename.split(/[/\\]/).includes('node_modules') ? undefined : true)
	},
	kit: {
		paths: { base: '/auth/v1' },
		adapter: adapter(
			dropin
				? { fallback: null, pages: '../templates/html', assets: '../static/v1', precompress: true, strict: true }
				: { fallback: null, pages: 'build/html', assets: 'build/static', precompress: false, strict: true }
		),
		alias: {
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
		},
		env: { publicPrefix: 'DEV_' }
	}
};

export default config;
