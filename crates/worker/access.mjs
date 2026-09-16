import { createRemoteJWKSet, jwtVerify } from 'jose';

export function accessSettings(env) {
  const domain = env.FOLIO_ACCESS_TEAM_DOMAIN ?? '';
  const audience = env.FOLIO_ACCESS_AUD ?? '';
  const emails = [...new Set((env.FOLIO_ACCESS_EMAIL ?? '').split(',').map(email => email.trim().toLowerCase()))];
  if (!/^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.cloudflareaccess\.com$/.test(domain) ||
      !/^[a-f0-9]{64}$/i.test(audience) || emails.some(email => !/^[^\s@,]+@[^\s@,]+\.[^\s@,]+$/.test(email))) {
    throw new Error('Configure the Access team domain, application AUD and allowed emails.');
  }
  return { issuer: `https://${domain}`, audience, emails };
}

export function createAccessVerifier(settings, keys = createRemoteJWKSet(new URL(`${settings.issuer}/cdn-cgi/access/certs`), {
  timeoutDuration: 5000, cooldownDuration: 30000, cacheMaxAge: 600000,
})) {
  return async token => {
    if (typeof token !== 'string' || token.length > 16384 || !token) throw new Error('Access token required');
    const { payload } = await jwtVerify(token, keys, {
      algorithms: ['RS256'], issuer: settings.issuer, audience: settings.audience,
      requiredClaims: ['exp', 'iat', 'sub', 'email'],
    });
    if (payload.type !== 'app' || typeof payload.sub !== 'string' || !payload.sub ||
        typeof payload.email !== 'string' || !settings.emails.includes(payload.email.toLowerCase()) ||
        !Number.isSafeInteger(payload.exp) || !Number.isSafeInteger(payload.exp * 1000)) {
      throw new Error('Access identity is not allowed');
    }
    return { subject: `${settings.issuer}|${payload.sub}`, username: payload.email, expires_at: payload.exp * 1000 };
  };
}

const jsonError = (status, message, method) => Response.json({ message, method }, {
  status, headers: { 'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff' },
});

export function createWorkerHandler(backend, verifierFactory = createAccessVerifier) {
  let cached;
  return {
    async fetch(request, env, ctx) {
      const started = performance.now();
      const method = env.FOLIO_AUTH_METHOD;
      // Always overwrite this internal binding: client headers and configured vars cannot supply identity.
      let identity = null;
      if (method === 'cloudflare_access') {
        let settings;
        try {
          settings = accessSettings(env);
          const key = JSON.stringify(settings);
          if (cached?.key !== key) cached = { key, verify: verifierFactory(settings) };
        } catch {
          return jsonError(503, 'Cloudflare Access is not configured.', method);
        }
        try { identity = await cached.verify(request.headers.get('Cf-Access-Jwt-Assertion')); }
        catch (error) {
          // Network/JWKS outages do not silently fall back to password authentication.
          const unavailable = error.code === 'ERR_JWKS_TIMEOUT' || error.code === 'ERR_JWKS_INVALID' || error.code === 'ERR_JOSE_GENERIC' || error instanceof TypeError;
          return jsonError(unavailable ? 503 : 401, unavailable ? 'Access verification is temporarily unavailable.' : 'Sign in with Cloudflare Access to continue.', method);
        }
      } else if (method !== 'password') {
        return jsonError(503, 'Configure FOLIO_AUTH_METHOD.', method);
      }
      const headers = new Headers(request.headers);
      headers.delete('Cf-Access-Jwt-Assertion');
      headers.delete('Cf-Access-Authenticated-User-Email');
      const trustedEnv = { ...env, FOLIO_INTERNAL_IDENTITY: JSON.stringify(identity) };
      const sanitized = new Request(request, { headers });
      const verified = performance.now();
      // worker-build 0.8.5 exports a WorkerEntrypoint class; retain its generated panic recovery proxy.
      const result = typeof backend === 'function'
        ? await new backend(ctx, trustedEnv).fetch(sanitized)
        : await backend.fetch(sanitized, trustedEnv, ctx);
      const authMs = Math.max(0, verified - started);
      const backendMs = Math.max(0, performance.now() - verified);
      const response = new Response(result.body, result);
      response.headers.set('Server-Timing', [result.headers.get('Server-Timing'), `access;dur=${authMs.toFixed(1)}, app;dur=${backendMs.toFixed(1)}`].filter(Boolean).join(', '));
      if (method === 'cloudflare_access') console.info(JSON.stringify({
        event: 'folio_timing', timing: response.headers.get('Server-Timing'),
        placement: request.headers.get('cf-placement') ?? 'default',
      }));
      return response;
    },
  };
}
