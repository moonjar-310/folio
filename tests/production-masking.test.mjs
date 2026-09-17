import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { productionMasks } from '../scripts/mask-production.mjs';

test('masks quoted env values, each allowed email and abbreviated email previews', () => {
  const masks = productionMasks('FOLIO_DOMAIN=folio.example.test\nFOLIO_ACCESS_EMAIL="alice@example.test, bob@example.test"\n', ['synthetic-secret']);
  for (const value of ['folio.example.test', 'alice@example.test', 'bob@example.test', 'alice', 'bob', 'synthetic-secret']) {
    assert.ok(masks.includes('::add-mask::' + value));
  }
});

test('mask command escapes command injection and never prints values outside Actions', () => {
  const commands = productionMasks('VALUE="line1\n::warning::injected%value"');
  assert.deepEqual(commands, ['::add-mask::line1%0A::warning::injected%25value']);
  const child = spawnSync(process.execPath, ['scripts/mask-production.mjs'], {
    env: { ...process.env, GITHUB_ACTIONS: 'false', FOLIO_PRODUCTION_ENV: 'SECRET=must-not-print' }, encoding: 'utf8',
  });
  assert.notEqual(child.status, 0);
  assert.equal(child.stdout, '');
  assert.ok(!child.stderr.includes('must-not-print'));
});

test('deployment registers masks before using secrets and suppresses binding summaries', () => {
  const workflow = readFileSync(new URL('../.github/workflows/cloudflare.yml', import.meta.url), 'utf8');
  const deploy = workflow.slice(workflow.indexOf('- name: Deploy main'));
  assert.match(deploy, /WRANGLER_LOG: warn/);
  const mask = deploy.indexOf('node scripts/mask-production.mjs');
  assert.ok(mask >= 0 && mask < deploy.indexOf('npm run worker:preflight'));
  assert.ok(mask < deploy.indexOf('> .env.production'));
});
