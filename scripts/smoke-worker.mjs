import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { unstable_dev } from 'wrangler';
import { exportJWK, generateKeyPair, SignJWT } from 'jose';
import { run } from './run-command.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const wrangler = resolve(root, 'node_modules/wrangler/bin/wrangler.js');
let worker;
const accessMode = process.argv.includes('--access');
let accessToken;
const authHeaders = () => accessToken ? { 'Cf-Access-Jwt-Assertion': accessToken } : {};
try {
  // worker:package creates .local and both release bundles before this script.
  const directory = await mkdtemp(resolve(root, '.local/worker-smoke-'));
  const config = resolve(directory, 'wrangler.toml');
  const persist = resolve(directory, 'state');
  let base = await readFile(resolve(root, 'wrangler.toml'), 'utf8');
  base = base.replace(/\[build\][\s\S]*?(?=\[assets\])/, '');
  let entry = resolve(root, 'crates/worker/entry.mjs');
  if (accessMode) {
    const pair = await generateKeyPair('RS256');
    const publicKey = { ...await exportJWK(pair.publicKey), kid: 'test-key', alg: 'RS256' };
    const team = 'folio-test.cloudflareaccess.com', audience = 'a'.repeat(64), email = 'owner@folio.dev';
    accessToken = await new SignJWT({ type: 'app', email }).setProtectedHeader({ alg: 'RS256', kid: 'test-key' })
      .setIssuer(`https://${team}`).setAudience(audience).setSubject('test-owner').setIssuedAt().setExpirationTime('30m').sign(pair.privateKey);
    // Test-only entrypoint with an ephemeral public key. Production always uses remote Access JWKS.
    entry = resolve(directory, 'entry.mjs');
    await writeFile(entry, `import rust from '../../crates/worker/build/worker/shim.mjs';\nimport { createWorkerHandler, createAccessVerifier } from '../../crates/worker/access.mjs';\nimport { createLocalJWKSet } from 'jose';\nconst keys = createLocalJWKSet(${JSON.stringify({ keys: [publicKey] })});\nexport default createWorkerHandler(rust, settings => createAccessVerifier(settings, keys));\n`);
    base = base.replace('FOLIO_AUTH_METHOD = "password"', `FOLIO_AUTH_METHOD = "cloudflare_access"\nFOLIO_ACCESS_TEAM_DOMAIN = "${team}"\nFOLIO_ACCESS_AUD = "${audience}"\nFOLIO_ACCESS_EMAIL = "${email}"`);
  }
  for (const path of ['crates/worker/entry.mjs', 'dist/web', 'migrations']) {
    base = base.replace(`"${path}"`, JSON.stringify((path === 'crates/worker/entry.mjs' ? entry : resolve(root, path)).replaceAll('\\', '/')));
  }
  await writeFile(config, base);
  await writeFile(resolve(directory, '.dev.vars'), 'FOLIO_SETUP_TOKEN=folio-local-worker-verification-token\nFOLIO_JWT_SECRET=' + 'test-emulator-only!'.repeat(4) + '\n');
  console.log(`Disposable local D1/R2 state: ${directory}`);
  await run(process.execPath, [wrangler, 'd1', 'migrations', 'apply', 'DB', '--local', '--config', config, '--persist-to', persist], {
    cwd: root, env: { ...process.env, CI: 'true', WRANGLER_SEND_METRICS: 'false' },
  });
  worker = await unstable_dev(entry, {
    config, local: true, ip: '127.0.0.1', port: 0, inspectorPort: 0,
    persistTo: persist, logLevel: 'error',
    experimental: { disableExperimentalWarning: true, disableDevRegistry: true, watch: false },
  });
  const url = `http://127.0.0.1:${worker.port}`;
  const get = path => fetch(url + path, { headers: authHeaders(), signal: AbortSignal.timeout(30_000) });
  if (accessMode) {
    assert.equal((await fetch(url + '/api/health')).status, 401);
    assert.equal((await fetch(url + '/api/notes', { headers: { 'Cf-Access-Jwt-Assertion': accessToken.slice(0, -10) + 'AAAAAAAAAA', 'Cf-Access-Authenticated-User-Email': 'owner@folio.dev' } })).status, 401);
  }
  const index = await get('/');
  assert.equal(index.status, 200);
  assert.match(index.headers.get('content-type'), /text\/html/);
  const html = await index.text();
  const wasm = html.match(/(?:href|src)="([^"]+\.wasm)"/)?.[1];
  assert.ok(wasm, 'Release HTML must reference browser WASM');
  const asset = await get(wasm.startsWith('/') ? wasm : '/' + wasm);
  assert.equal(asset.status, 200);
  assert.match(asset.headers.get('content-type'), /application\/wasm/);
  assert.deepEqual([...new Uint8Array(await asset.arrayBuffer()).slice(0, 4)], [0, 97, 115, 109]);
  assert.equal(await (await get('/notes/deep-link')).text(), html, 'SPA deep links must serve the app');
  const health = await get('/api/health');
  assert.deepEqual(await health.json(), { status: 'ok', runtime: 'cloudflare' });
  assert.equal(health.headers.get('cache-control'), 'no-store');
  for (const path of ['/api', '/api/missing']) {
    const api = await fetch(url + path, { headers: { ...authHeaders(), 'sec-fetch-mode': 'navigate' } });
    assert.match(api.headers.get('content-type'), /application\/json/, 'API navigation must reach Worker, not SPA fallback');
    assert.ok(api.status >= 400);
  }
  const blocked = await fetch(url + '/api/auth/setup', {
    method: 'POST', headers: { ...authHeaders(), 'content-type': 'application/json', 'x-setup-token': 'incorrect' },
    body: JSON.stringify({ username: 'folio-test', password: 'Folio-local-test-2026!' }),
  });
  assert.equal(blocked.status, accessMode ? 404 : 403, 'Setup is closed without password-mode setup credentials');
  await run(process.execPath, ['scripts/smoke-product.mjs', url], { cwd: root, env: { ...process.env, FOLIO_TEST_JWT_SECRET: 'test-emulator-only!'.repeat(4), ...(accessToken ? { FOLIO_TEST_ACCESS_TOKEN: accessToken } : {}) } });
  console.log(`PASS: Cloudflare static HTML/WASM, SPA/API routing, ${accessMode ? 'signed Access JWT' : 'Argon2id password'}, and D1/R2 product workflow.`);
} catch (error) {
  console.error(error);
  process.exitCode = 1;
} finally {
  await worker?.stop();
}
