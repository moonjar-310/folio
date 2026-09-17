# Authentication implementation

## Shared token contract

Both runtimes issue a Folio HS256 Access JWT valid for **300 seconds** and a random Refresh Token valid for **604800 seconds (one week)**. The browser keeps the Access JWT and CSRF value in memory, sends the JWT as `Authorization: Bearer`, and refreshes before an API call when the JWT has ten seconds or less remaining. There is no background refresh timer. A shared browser-instance mutex serializes refresh requests. An unexpected API 401 triggers one refresh and one retry, never a retry loop. Reloading restores authentication through `POST /api/auth/refresh`; tokens are never written to localStorage or sessionStorage.

The Refresh Token is an HttpOnly, SameSite=Strict cookie. HTTPS uses `__Host-folio_refresh`, Secure, Path=/ and no Domain; loopback HTTP uses `folio_refresh`. Only its SHA-256 digest is stored in `sessions`, alongside username, provider, subject, CSRF and absolute expiry. UUID v4 randomness comes from the runtime CSPRNG. A refresh atomically updates the digest with SQL UPDATE … RETURNING, so the old token cannot be reused, including concurrent requests. Rotation preserves the original one-week deadline. Expired rows are removed at login. Refresh requires an explicit same-origin Origin header and JSON content type.

Business request authorization checks the JWT signature in constant time, the fixed HS256 header, issuer, audience, provider, subject, issued-at and expiry. It never queries the sessions table. Mutations also validate the JWT-bound CSRF value and reject foreign Origins. Refresh Tokens cannot authorize business APIs.

Logout deletes the current Refresh Token row and expires its cookie. An already issued Access JWT remains valid for at most five minutes; deleting session rows does not immediately revoke it. One week after login, a new login is required. Refresh replay is rejected; it does not revoke an entire token family. Simultaneous refreshes in separate tabs can cause one tab to require reauthentication. Draft preservation remains in place.

## Login providers

Local password login uses Argon2id v19, 19 MiB memory, two iterations, parallelism one. A legacy PBKDF2 hash upgrades only after successful password verification. Native runtime generates a persistent signing key under its private data directory (`jwt-secret`, mode 0600 on Unix); `FOLIO_JWT_SECRET` can override it. The key must be at least 64 characters. Losing or changing the key invalidates outstanding Access JWTs; existing refresh sessions can issue new JWTs.

Production login uses Cloudflare Access. The JavaScript adapter verifies its RS256 assertion, issuer, application AUD, expiry, identity and exact email allowlist with jose/Web Crypto. JWKS public keys are cached for ten minutes (30-second reload cooldown, five-second timeout). The adapter overwrites the internal VerifiedIdentity binding and removes incoming identity assertion headers before invoking Rust. Folio exchanges this verified identity for its own token pair at login. Production also binds each app token to the current verified Access subject.

Cloudflare Access still protects the whole hostname, including refresh. Its independent login policy/session lifetime can require an email PIN before Folio's one-week refresh expiry. Folio refresh never bypasses that policy. workers.dev and preview URLs stay disabled. Logout navigates to `/cdn-cgi/access/logout` after Folio logout. Allowed emails share one vault; per-user vaults and public signup are not implemented.

Production signs tokens with the `FOLIO_JWT_SECRET` Worker secret, also held as an encrypted GitHub Actions secret for deployment. It must not be placed in tracked files, TOML vars, browser bundles or logs. No app password hashing happens in Access mode.

## Runtime boundaries and verification

Token issuance, refresh, verification and business APIs are common Rust application code; runtime adapters supply headers, signing keys, storage and the external login identity. Tests cover exact expiry boundaries, one-week absolute refresh expiry, hash-only storage, replay rejection, missing refresh Origin, changed identities, invalid signatures and CSRF, and successful authorization with the sessions table inaccessible. Worker smoke tests execute both login providers through real Rust/D1/R2 APIs. Live email delivery and Cloudflare policy are separate from emulator coverage.
