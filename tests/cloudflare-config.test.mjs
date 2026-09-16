import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { mkdtemp, writeFile, unlink, rmdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { unstable_readConfig } from 'wrangler';
import { productionConfig } from '../scripts/cloudflare-config.mjs';

const base = readFileSync(new URL('../wrangler.toml', import.meta.url), 'utf8').replaceAll('\r\n', '\n');
const valid = {
  CLOUDFLARE_ACCOUNT_ID: '1234567890abcdef1234567890abcdef',
  FOLIO_D1_DATABASE_ID: '12345678-1234-1234-1234-123456789abc',
  FOLIO_DOMAIN: 'folio.owned-domain.dev',
  FOLIO_ACCESS_TEAM_DOMAIN: 'folio-test.cloudflareaccess.com',
  FOLIO_ACCESS_AUD: 'a'.repeat(64),
  FOLIO_ACCESS_EMAIL: 'owner@owned-domain.dev',
};
test('production retains shared build and bindings while disabling alternate public URLs', () => {
  const result = productionConfig(base, valid);
  assert.ok(result.includes(`account_id = "${valid.CLOUDFLARE_ACCOUNT_ID}"`));
  assert.ok(result.includes(`database_id = "${valid.FOLIO_D1_DATABASE_ID}"`));
  assert.match(result, /custom_domain = true/);
  assert.match(result, /workers_dev = false/);
  assert.match(result, /preview_urls = false/);
  assert.ok(!result.includes('cpu_ms'));
  assert.match(result, /FOLIO_AUTH_METHOD = "cloudflare_access"/);
  for (const section of ['[build]', '[assets]', '[[r2_buckets]]']) {
    const shared = base.slice(base.indexOf(section)).split(/\n\[/)[0];
    assert.ok(result.includes(shared));
  }
  assert.ok(!result.includes('00000000-0000-0000-0000-000000000000'));
});
test('missing IDs, local placeholders and unsafe/placeholder hosts fail before output', () => {
  for (const key of Object.keys(valid)) assert.throws(() => productionConfig(base, { ...valid, [key]: '' }));
  assert.throws(() => productionConfig(base, { ...valid, FOLIO_D1_DATABASE_ID: '00000000-0000-0000-0000-000000000000' }));
  assert.throws(() => productionConfig(base, { ...valid, CLOUDFLARE_ACCOUNT_ID: '0'.repeat(32) }));
  for (const domain of ['https://folio.test.dev', '*.test.dev', 'host.dev/path', 'localhost', 'folio.example.com', 'foo.invalid', 'a".dev', 'a\n.dev', '127.0.0.1', '-bad.dev']) {
    assert.throws(() => productionConfig(base, { ...valid, FOLIO_DOMAIN: domain }), domain);
  }
});
test('base config drift fails instead of silently generating a wrong production binding', () => {
  assert.throws(() => productionConfig(base.replace('database_id =', 'id ='), valid));
  assert.throws(() => productionConfig(base + '\n[limits]\ncpu_ms = 10', valid));
});

test('explicit storage placement is optional and rejects configuration injection', () => {
  assert.match(productionConfig(base, { ...valid, FOLIO_PLACEMENT_REGION: 'aws:ap-east-1' }), /region = "aws:ap-east-1"/);
  assert.throws(() => productionConfig(base, { ...valid, FOLIO_PLACEMENT_REGION: 'aws:ap-east-1"\n[vars]' }));
});
test('Wrangler accepts generated TOML and sees production routing, CPU and storage', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'folio-config-'));
  const path = join(directory, 'wrangler.toml');
  try {
    await writeFile(path, productionConfig(base, valid));
    const config = unstable_readConfig({ config: path });
    assert.equal(config.account_id, valid.CLOUDFLARE_ACCOUNT_ID);
    assert.deepEqual(config.routes, [{ pattern: valid.FOLIO_DOMAIN, custom_domain: true }]);
    assert.equal(config.d1_databases[0].database_id, valid.FOLIO_D1_DATABASE_ID);
    assert.equal(config.r2_buckets[0].bucket_name, 'folio-notes');
    assert.equal(config.assets.binding, 'ASSETS');
    assert.equal(config.limits?.cpu_ms, undefined);
    assert.equal(config.vars.FOLIO_AUTH_METHOD, 'cloudflare_access');
    assert.equal(config.placement.mode, 'smart');
    assert.equal(config.vars.FOLIO_ACCESS_EMAIL, valid.FOLIO_ACCESS_EMAIL);
  } finally {
    await unlink(path);
    await rmdir(directory);
  }
});
