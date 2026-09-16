import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { productionConfig } from './cloudflare-config.mjs';
import { run } from './run-command.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const actions = {
  preflight: ['deploy', '--dry-run', '--outdir', '.local/worker-production-check'],
  deploy: ['deploy'],
  migrate: ['d1', 'migrations', 'apply', 'DB', '--remote'],
};
try {
  const action = process.argv[2] ?? 'configure';
  if (action !== 'configure' && !Object.hasOwn(actions, action)) throw new Error('Expected configure, preflight, migrate, or deploy.');
  const base = await readFile(new URL('../wrangler.toml', import.meta.url), 'utf8');
  const config = productionConfig(base.replaceAll('\r\n', '\n'), process.env);
  await writeFile(new URL('../wrangler.production.toml', import.meta.url), config);
  console.log('Production configuration written to ignored wrangler.production.toml (Cloudflare Access, D1, R2, Custom Domain).');
  if (action !== 'configure') {
    await run(process.execPath, ['node_modules/wrangler/bin/wrangler.js', ...actions[action], '--config', 'wrangler.production.toml'], { cwd: root });
  }
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
