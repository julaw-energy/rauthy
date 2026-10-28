import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
// @ts-ignore plain JS module
import { checkPage } from '../tools/check-build.mjs';

const page = (csp: string, scripts: string) =>
	`<html><head><meta http-equiv="content-security-policy" content="${csp}">${scripts}</head></html>`;
const exists = () => true;

test('a page with a hashed inline script and a local script file is fine', () => {
	const body = 'console.log(1)';
	const h = createHash('sha256').update(body).digest('base64');
	const html = page(
		`default-src 'none'; script-src 'self' 'wasm-unsafe-eval' 'sha256-${h}'`,
		`<script>${body}</script><script src="/auth/v1/a.js"></script>`,
	);
	assert.deepEqual(checkPage('p.html', html, exists), []);
});

test('a script inlined as a data: URI is refused (the bug of custom.4)', () => {
	const html = page(`script-src 'self'`, `<script src="data:text/javascript;base64,AAAA"></script>`);
	const out = checkPage('p.html', html, exists);
	assert.equal(out.length, 1);
	assert.match(out[0], /data: URI/);
});

test('an inline script without a hash, another origin, a missing file and a loose policy are refused', () => {
	assert.match(checkPage('p', page(`script-src 'self'`, '<script>x()</script>'), exists)[0], /no hash/);
	assert.match(
		checkPage('p', page(`script-src 'self'`, '<script src="https://cdn.example/x.js"></script>'), exists)[0],
		/another origin/,
	);
	assert.match(
		checkPage('p', page(`script-src 'self'`, '<script src="/auth/v1/gone.js"></script>'), () => false)[0],
		/does not exist/,
	);
	assert.match(checkPage('p', page(`script-src 'self' 'unsafe-inline'`, ''), exists)[0], /unsafe-inline/);
	assert.match(checkPage('p', '<html></html>', exists)[0], /no Content-Security-Policy/);
});
