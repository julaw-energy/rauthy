// Checks the built pages against the Content-Security-Policy they carry, so a script the browser would
// refuse fails the BUILD instead of showing up as a console error in the deployed image.
//
//   node tools/check-build.mjs <pages dir> <assets dir>
//
// Runs after `pnpm build` and `pnpm build:dropin` (see the `post*` scripts in package.json), so the image
// build stops too. Plain node, no dependencies.
import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const BASE = '/auth/v1/';

/** Returns the problems of one page, an empty list when it is fine */
export function checkPage(name, html, fileExists) {
	const problems = [];
	const fail = msg => problems.push(`${name}: ${msg}`);

	const meta = html.match(/<meta[^>]*http-equiv="content-security-policy"[^>]*content="([^"]*)"/i);
	if (!meta) {
		fail('no Content-Security-Policy meta tag');
		return problems;
	}
	const directives = Object.fromEntries(
		meta[1].split(';').map(d => {
			const [k, ...v] = d.trim().split(/\s+/);
			return [k, v];
		}),
	);
	const scriptSrc = directives['script-src'] ?? directives['default-src'] ?? [];
	for (const bad of ["'unsafe-inline'", "'unsafe-eval'", '*', 'https:', 'data:']) {
		if (scriptSrc.includes(bad)) fail(`script-src allows ${bad}`);
	}

	for (const m of html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)) {
		const [, attrs, body] = m;
		const src = attrs.match(/\bsrc="([^"]*)"/i)?.[1];
		if (src === undefined) {
			// inline: the browser only runs it when its hash is in the policy
			const hash = `'sha256-${createHash('sha256').update(body).digest('base64')}'`;
			if (!scriptSrc.includes(hash)) fail(`an inline script has no hash in the policy (${hash})`);
			continue;
		}
		if (/^(data|blob|javascript):/i.test(src)) {
			fail(`a script is loaded from a ${src.split(':')[0]}: URI, script-src 'self' refuses it (${src.slice(0, 60)}...)`);
		} else if (/^(https?:)?\/\//i.test(src)) {
			fail(`a script is loaded from another origin: ${src}`);
		} else if (src.startsWith(BASE) && !fileExists(src.slice(BASE.length))) {
			fail(`the script ${src} does not exist in the build`);
		}
	}
	return problems;
}

function main() {
	const [pages, assets] = process.argv.slice(2);
	if (!pages || !assets) {
		console.error('usage: node tools/check-build.mjs <pages dir> <assets dir>');
		process.exit(2);
	}
	const files = readdirSync(pages, { recursive: true }).filter(f => String(f).endsWith('.html'));
	if (files.length === 0) {
		console.error(`check-build: no pages found in ${pages}`);
		process.exit(1);
	}
	const problems = files.flatMap(f =>
		checkPage(f, readFileSync(join(pages, f), 'utf8'), p => existsSync(join(assets, p))),
	);
	if (problems.length) {
		console.error(`check-build: ${problems.length} problem(s) that would break in the browser:`);
		for (const p of problems) console.error(`  - ${p}`);
		process.exit(1);
	}
	console.log(`check-build: ${files.length} pages ok`);
}

if (import.meta.url === `file://${process.argv[1]}`) main();
