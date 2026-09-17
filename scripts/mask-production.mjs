import { parseEnv } from 'node:util';
import { pathToFileURL } from 'node:url';

export function productionMasks(source, extra = []) {
  const values = Object.values(parseEnv(source));
  const masks = new Set([...values, ...extra].filter(Boolean));
  for (const value of values) {
    // A multi-email env value is shortened in Wrangler's binding summary.
    // Mask each identity independently, including its local part in previews.
    for (const item of value.split(',')) {
      const email = item.trim();
      if (email.includes('@')) {
        masks.add(email);
        const local = email.slice(0, email.indexOf('@'));
        if (local.length >= 3) masks.add(local);
      }
    }
  }
  return [...masks].sort((a, b) => b.length - a.length).map(value =>
    '::add-mask::' + value.replaceAll('%', '%25').replaceAll('\r', '%0D').replaceAll('\n', '%0A'));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  if (process.env.GITHUB_ACTIONS !== 'true' || !process.env.FOLIO_PRODUCTION_ENV) {
    console.error('Production masking requires GitHub Actions and production configuration.');
    process.exitCode = 1;
  } else {
    try {
      const commands = productionMasks(process.env.FOLIO_PRODUCTION_ENV,
        [process.env.CLOUDFLARE_API_TOKEN, process.env.FOLIO_JWT_SECRET]);
      for (const command of commands) process.stdout.write(command + '\n');
    } catch {
      // Never echo a parser error that may include production input.
      console.error('Could not register production log masks.');
      process.exitCode = 1;
    }
  }
}
