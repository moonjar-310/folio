import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { run } from './run-command.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const version = '0.8.5';
try {
  // Build both halves even from a clean checkout: Wrangler serves this CSR bundle.
  await run(process.execPath, ['scripts/trunk.mjs', 'build', '--release', '--locked'], { cwd: root });
  const installed = spawnSync('worker-build', ['--version'], { encoding: 'utf8' });
  if (installed.status !== 0 || installed.stdout.trim() !== version) {
    await run('cargo', ['install', '--locked', 'worker-build', '--version', version], { cwd: root });
  }
  await run('worker-build', ['--release', '--', '--locked'], {
    cwd: fileURLToPath(new URL('../crates/worker/', import.meta.url)),
  });
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
