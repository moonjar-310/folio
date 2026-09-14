import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

// Trunk's boolean parser rejects the common NO_COLOR=1 convention.
const env = { ...process.env };
if (env.NO_COLOR && env.NO_COLOR !== 'false') env.NO_COLOR = 'true';
const child = spawn('trunk', process.argv.slice(2), {
  cwd: fileURLToPath(new URL('../crates/web/', import.meta.url)),
  env,
  stdio: 'inherit',
});
child.on('error', (error) => { console.error(error.message); process.exitCode = 1; });
child.on('exit', (code) => { process.exitCode = code ?? 1; });
