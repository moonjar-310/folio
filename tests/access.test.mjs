import assert from 'node:assert/strict';
import test from 'node:test';
import { createLocalJWKSet, createRemoteJWKSet, customFetch, exportJWK, generateKeyPair, SignJWT } from 'jose';
import { accessSettings, createAccessVerifier, createWorkerHandler } from '../crates/worker/access.mjs';

const env = { FOLIO_AUTH_METHOD: 'cloudflare_access', FOLIO_ACCESS_TEAM_DOMAIN: 'folio-test.cloudflareaccess.com', FOLIO_ACCESS_AUD: 'a'.repeat(64), FOLIO_ACCESS_EMAIL: 'owner@folio.dev' };
const settings = accessSettings(env);
const pair = await generateKeyPair('RS256');
const jwk = { ...await exportJWK(pair.publicKey), kid: 'first', alg: 'RS256' };
const keys = createLocalJWKSet({ keys: [jwk] });
const verifier = createAccessVerifier(settings, keys);
const now = Math.floor(Date.now() / 1000);
const sign = (overrides = {}, privateKey = pair.privateKey, kid = 'first') => new SignJWT({
  iss: settings.issuer, aud: settings.audience, sub: 'owner-subject', email: settings.emails[0], type: 'app', iat: now, exp: now + 600, ...overrides,
}).setProtectedHeader({ alg: 'RS256', kid }).sign(privateKey);

test('signed Access identity and rejection of untrusted claims/signatures', async () => {
  const token = await sign();
  assert.deepEqual(await verifier(token), { subject: `${settings.issuer}|owner-subject`, username: settings.emails[0], expires_at: (now + 600) * 1000 });
  for (const claims of [{ iss: 'https://evil.test' }, { aud: 'b'.repeat(64) }, { exp: now - 1 }, { nbf: now + 60 }, { email: 'other@folio.dev' }, { sub: '' }, { type: 'service' }, { exp: undefined }]) {
    await assert.rejects(() => sign(claims).then(verifier));
  }
  const other = await generateKeyPair('RS256');
  await assert.rejects(() => sign({}, other.privateKey).then(verifier));
  const hmac = await new SignJWT({ iss: settings.issuer, aud: settings.audience, sub: 'owner', email: settings.emails[0], type: 'app', iat: now, exp: now + 600 })
    .setProtectedHeader({ alg: 'HS256' }).sign(new Uint8Array(32));
  await assert.rejects(() => verifier(hmac));
  for (const token of [null, '', 'bad.token.value', 'a'.repeat(17000)]) await assert.rejects(() => verifier(token));
});

test('entrypoint verifies each request, overwrites identity and never falls back on failure', async () => {
  let calls = 0;
  const backend = { fetch: async (req, bindings) => { calls++; return Response.json({ identity: JSON.parse(bindings.FOLIO_INTERNAL_IDENTITY), assertion: req.headers.get('Cf-Access-Jwt-Assertion') }); } };
  const worker = createWorkerHandler(backend, () => verifier);
  const request = token => new Request('https://folio.dev/api/notes', { headers: { ...(token ? { 'Cf-Access-Jwt-Assertion': token } : {}), 'Cf-Access-Authenticated-User-Email': settings.emails[0], FOLIO_INTERNAL_IDENTITY: 'forged' } });
  assert.equal((await worker.fetch(request(), { ...env, FOLIO_INTERNAL_IDENTITY: 'forged' }, {})).status, 401);
  assert.equal(calls, 0);
  const response = await worker.fetch(request(await sign()), env, {});
  assert.equal((await response.json()).identity.username, settings.emails[0]);
  assert.equal((await worker.fetch(request(), { ...env, FOLIO_AUTH_METHOD: 'unknown' }, {})).status, 503);
  assert.equal((await worker.fetch(request(), { ...env, FOLIO_ACCESS_TEAM_DOMAIN: 'evil.test' }, {})).status, 503);
  const local = await worker.fetch(request(await sign()), { ...env, FOLIO_AUTH_METHOD: 'password', FOLIO_INTERNAL_IDENTITY: 'forged' }, {});
  assert.deepEqual(await local.json(), { identity: null, assertion: null });
  class RustEntrypoint {
    constructor(ctx, bindings) { this.bindings = bindings; }
    fetch() { return Response.json(JSON.parse(this.bindings.FOLIO_INTERNAL_IDENTITY)); }
  }
  const classWorker = createWorkerHandler(RustEntrypoint, () => verifier);
  assert.equal((await (await classWorker.fetch(request(await sign()), env, {})).json()).username, settings.emails[0]);
  const outage = createWorkerHandler(backend, () => async () => { throw new TypeError('network'); });
  assert.equal((await outage.fetch(request(await sign()), env, {})).status, 503);
});

test('remote key set caches keys and reloads for rotation', async () => {
  const second = await generateKeyPair('RS256');
  const secondJwk = { ...await exportJWK(second.publicKey), kid: 'second', alg: 'RS256' };
  let fetches = 0;
  let published = [jwk];
  const remote = createRemoteJWKSet(new URL(`${settings.issuer}/cdn-cgi/access/certs`), {
    cooldownDuration: 0,
    [customFetch]: async () => { fetches++; return Response.json({ keys: published }); },
  });
  const verify = createAccessVerifier(settings, remote);
  await verify(await sign()); await verify(await sign()); assert.equal(fetches, 1);
  published = [jwk, secondJwk];
  await verify(await sign({}, second.privateKey, 'second'));
  assert.equal(fetches, 2);
});


test('email allowlist accepts either identity and rejects outsiders and malformed lists', async () => {
  const multiple = accessSettings({ ...env, FOLIO_ACCESS_EMAIL: ' Owner@folio.dev, second@folio.dev,owner@folio.dev ' });
  assert.deepEqual(multiple.emails, ['owner@folio.dev', 'second@folio.dev']);
  const verify = createAccessVerifier(multiple, keys);
  for (const email of ['owner@folio.dev', 'second@folio.dev', 'SECOND@folio.dev']) {
    assert.equal((await verify(await sign({ email }))).username, email);
  }
  await assert.rejects(() => sign({ email: 'outsider@folio.dev' }).then(verify));
  for (const value of ['', 'owner@folio.dev,', 'owner@folio.dev,bad', '*@*']) {
    assert.throws(() => accessSettings({ ...env, FOLIO_ACCESS_EMAIL: value }));
  }
});
