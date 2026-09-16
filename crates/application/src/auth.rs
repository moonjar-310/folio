use super::*;
use subtle::ConstantTimeEq;
const SESSION_MS: u64 = 7 * 24 * 60 * 60 * 1000;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthMethod {
    Password,
    CloudflareAccess,
}
impl AuthMethod {
    pub fn name(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::CloudflareAccess => "cloudflare_access",
        }
    }
}
/// Verified by the runtime adapter. This is not a client API input type.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct VerifiedIdentity {
    pub subject: String,
    pub username: String,
    pub expires_at: u64,
}
fn equal(a: &str, b: &str) -> bool {
    bool::from(a.as_bytes().ct_eq(b.as_bytes()))
}
fn cookie(token: &str, secure: bool, max_age: u64) -> String {
    format!(
        "{}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age}{}",
        if secure {
            "__Host-folio_refresh"
        } else {
            "folio_refresh"
        },
        if secure { "; Secure" } else { "" }
    )
}
impl<S: Store> Application<S> {
    fn external_identity<'a>(&self, req: &'a Request) -> Result<&'a VerifiedIdentity> {
        req.identity
            .as_ref()
            .filter(|id| {
                !id.subject.is_empty() && !id.username.is_empty() && id.expires_at > self.now
            })
            .ok_or_else(|| fail(401, "Sign in with Cloudflare Access to continue."))
    }
    pub(crate) async fn authorize(&mut self, req: &Request) -> Result<Value> {
        let claims = crate::tokens::verify(
            &self.jwt_secret,
            &req.access_token,
            self.now / 1000,
            self.auth_method.name(),
        )?;
        if self.auth_method == AuthMethod::CloudflareAccess
            && self.external_identity(req)?.subject != claims.sub
        {
            return Err(fail(401, "Identity changed. Sign in again."));
        }
        if req.method != "GET" && !equal(&claims.csrf, &req.csrf) {
            return Err(fail(403, "Invalid security token. Sign in again."));
        }
        Ok(json!({"csrf":claims.csrf,"username":claims.username}))
    }
    pub(crate) async fn auth(&mut self, req: &Request, input: Value) -> Result<Response> {
        if req.path == "/api/auth/refresh" && req.method == "POST" {
            return self.refresh(req).await;
        }
        if self.auth_method == AuthMethod::CloudflareAccess {
            return self.access_auth(req).await;
        }
        let accounts = self
            .store
            .query(statement(
                "SELECT username,salt,password_hash FROM account WHERE id=1",
                vec![],
            ))
            .await?;
        if req.path == "/api/auth/status" && req.method == "GET" {
            let session = self.authorize(req).await.ok();
            let authenticated = session.is_some();
            return Ok(Response::ok(
                json!({"method":self.auth_method.name(),"setup_required":accounts.is_empty(),"authenticated":authenticated,"csrf":session.map(|s|s["csrf"].clone()),"username":accounts.first().filter(|_|authenticated).map(|a|a["username"].clone()),"runtime":self.runtime}),
            ));
        }
        if req.path == "/api/auth/logout" && req.method == "POST" {
            return self.logout(req).await;
        }
        if req.method != "POST"
            || !["/api/auth/setup", "/api/auth/login"].contains(&req.path.as_str())
        {
            return Err(fail(404, "API route not found"));
        }
        let username = string(&input, "username").trim();
        let password = string(&input, "password");
        if username.is_empty() || username.len() > 80 || password.len() > 1024 {
            return Err(fail(400, "Enter a username and password."));
        }
        if req.path.ends_with("/setup") {
            if !accounts.is_empty() {
                return Err(fail(409, "This workspace is already configured."));
            }
            if self.runtime == "cloudflare" {
                let secret = self
                    .setup_secret
                    .as_ref()
                    .filter(|s| s.len() >= 24)
                    .ok_or_else(|| {
                        fail(
                            503,
                            "Configure the FOLIO_SETUP_TOKEN Worker secret before initial setup.",
                        )
                    })?;
                if !equal(secret, &req.setup_token) {
                    return Err(fail(403, "The setup token is incorrect."));
                }
            }
            if password.len() < 12 {
                return Err(fail(400, "Use a password with at least 12 characters."));
            }
            let hash = crate::password::hash(password)?;
            self.store
                .execute(statement(
                    "INSERT INTO account(id,username,salt,password_hash) VALUES(1,?1,?2,?3)",
                    vec![json!(username), json!(""), json!(hash)],
                ))
                .await?;
        } else {
            // Atomic SQL counters apply before expensive password verification.
            let key = digest(&req.client);
            self.store.execute(statement("INSERT INTO login_attempts(id,count,reset_at) VALUES(?1,1,?2) ON CONFLICT(id) DO UPDATE SET count=CASE WHEN reset_at<=?3 THEN 1 ELSE count+1 END, reset_at=CASE WHEN reset_at<=?3 THEN ?2 ELSE reset_at END",vec![json!(key),json!(self.now+15*60*1000),json!(self.now)])).await?;
            let attempts = self
                .store
                .query(statement(
                    "SELECT count FROM login_attempts WHERE id=?1",
                    vec![json!(key)],
                ))
                .await?;
            if attempts[0]["count"].as_u64().unwrap_or(100) > 10 {
                return Err(fail(429, "Too many attempts. Try again in 15 minutes."));
            }
            let account = accounts
                .first()
                .ok_or_else(|| fail(401, "Incorrect username or password."))?;
            let verification = crate::password::verify(
                password,
                string(account, "salt"),
                string(account, "password_hash"),
            );
            if !equal(username, string(account, "username")) || verification.is_none() {
                return Err(fail(401, "Incorrect username or password."));
            }
            if verification == Some(true) {
                let upgraded = crate::password::hash(password)?;
                self.store.execute(statement("UPDATE account SET password_hash=?1,salt='' WHERE id=1 AND password_hash=?2", vec![json!(upgraded), account["password_hash"].clone()])).await?;
            }
            self.store
                .execute(statement(
                    "DELETE FROM login_attempts WHERE id=?1 OR reset_at<=?2",
                    vec![json!(key), json!(self.now)],
                ))
                .await?;
        }
        self.issue_session(req, username, "local-owner", self.now + SESSION_MS)
            .await
    }

    async fn access_auth(&mut self, req: &Request) -> Result<Response> {
        let identity = self.external_identity(req)?.clone();
        if req.method == "GET" && req.path == "/api/auth/status" {
            let session = self.authorize(req).await.ok();
            return Ok(Response::ok(
                json!({"method":self.auth_method.name(),"setup_required":false,"authenticated":session.is_some(),"csrf":session.as_ref().map(|s|s["csrf"].clone()),"username":session.map(|s|s["username"].clone()),"runtime":self.runtime}),
            ));
        }
        if req.method == "POST" && req.path == "/api/auth/login" {
            return self
                .issue_session(
                    req,
                    &identity.username,
                    &identity.subject,
                    self.now + SESSION_MS,
                )
                .await;
        }
        if req.method == "POST" && req.path == "/api/auth/logout" {
            return self.logout(req).await;
        }
        Err(fail(
            404,
            "Password setup is unavailable with Cloudflare Access.",
        ))
    }

    async fn refresh(&mut self, req: &Request) -> Result<Response> {
        // Refresh is a cookie-authenticated mutation. Require an explicit same-origin request,
        // including on startup when the in-memory access token and CSRF are unavailable.
        if req.origin.as_deref() != Some(req.expected_origin.as_str())
            || req.expected_origin.is_empty()
        {
            return Err(fail(403, "Cross-origin refresh rejected"));
        }
        if req.session.len() != 72 {
            return Err(fail(401, "Please sign in again."));
        }
        let subject = if self.auth_method == AuthMethod::CloudflareAccess {
            self.external_identity(req)?.subject.clone()
        } else {
            "local-owner".into()
        };
        let token = format!("{}{}", uid(), uid());
        // One atomic compare-and-swap consumes the old token, including concurrent refreshes.
        let rows = self.store.query(statement(
            "UPDATE sessions SET id=?1 WHERE id=?2 AND expires_at>?3 AND provider=?4 AND subject=?5 RETURNING csrf,expires_at,username",
            vec![json!(digest(&token)),json!(digest(&req.session)),json!(self.now),json!(self.auth_method.name()),json!(subject)]
        )).await?;
        let row = rows.first().ok_or_else(|| {
            fail(
                401,
                "Refresh token expired or already used. Please sign in again.",
            )
        })?;
        self.token_response(
            req,
            &token,
            string(row, "csrf"),
            string(row, "username"),
            &subject,
            row["expires_at"].as_u64().unwrap_or(0),
        )
    }

    async fn logout(&mut self, req: &Request) -> Result<Response> {
        self.authorize(req).await?;
        self.store
            .execute(statement(
                "DELETE FROM sessions WHERE id=?1",
                vec![json!(digest(&req.session))],
            ))
            .await?;
        Ok(Response {
            status: 200,
            body: json!({"ok":true,"logout_url":if self.auth_method == AuthMethod::CloudflareAccess { Some("/cdn-cgi/access/logout") } else { None }}),
            cookie: Some(cookie("", req.secure, 0)),
        })
    }

    async fn issue_session(
        &mut self,
        req: &Request,
        username: &str,
        subject: &str,
        expires_at: u64,
    ) -> Result<Response> {
        let token = format!("{}{}", uid(), uid());
        let csrf = uid();
        let response = self.token_response(req, &token, &csrf, username, subject, expires_at)?;
        self.store.batch(vec![
            statement("DELETE FROM sessions WHERE expires_at<=?1", vec![json!(self.now)]),
            statement("INSERT INTO sessions(id,csrf,expires_at,provider,subject,username) VALUES(?1,?2,?3,?4,?5,?6)",
                vec![json!(digest(&token)),json!(csrf),json!(expires_at),json!(self.auth_method.name()),json!(subject),json!(username)]),
        ]).await?;
        Ok(response)
    }

    fn token_response(
        &self,
        req: &Request,
        refresh: &str,
        csrf: &str,
        username: &str,
        subject: &str,
        expires_at: u64,
    ) -> Result<Response> {
        let claims = crate::tokens::Claims {
            iss: "folio".into(),
            aud: "folio-api".into(),
            sub: subject.into(),
            username: username.into(),
            provider: self.auth_method.name().into(),
            csrf: csrf.into(),
            iat: self.now / 1000,
            exp: self.now / 1000 + crate::tokens::ACCESS_SECONDS,
        };
        let token = crate::tokens::sign(&self.jwt_secret, &claims)?;
        Ok(Response {
            status: 200,
            body: json!({"method":self.auth_method.name(),"authenticated":true,"csrf":csrf,"username":username,"runtime":self.runtime,"access_token":token,"expires_in":crate::tokens::ACCESS_SECONDS}),
            cookie: Some(cookie(
                refresh,
                req.secure,
                expires_at.saturating_sub(self.now) / 1000,
            )),
        })
    }
}
