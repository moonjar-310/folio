use super::*;
use subtle::ConstantTimeEq;
const SESSION_MS: u64 = 7 * 24 * 60 * 60 * 1000;
fn password_hash(password: &str, salt: &str) -> String {
    let mut output = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), salt.as_bytes(), 600_000, &mut output);
    output.iter().map(|b| format!("{b:02x}")).collect()
}
fn equal(a: &str, b: &str) -> bool {
    bool::from(a.as_bytes().ct_eq(b.as_bytes()))
}
fn cookie(token: &str, secure: bool, max_age: u64) -> String {
    format!(
        "folio_session={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age}{}",
        if secure { "; Secure" } else { "" }
    )
}
impl<S: Store> Application<S> {
    pub(crate) async fn authorize(&mut self, req: &Request) -> Result<Value> {
        if req.session.is_empty() {
            return Err(fail(401, "Please sign in to your workspace."));
        }
        let session = self
            .store
            .query(statement(
                "SELECT csrf FROM sessions WHERE id=?1 AND expires_at>?2",
                vec![json!(digest(&req.session)), json!(self.now)],
            ))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| fail(401, "Your session expired. Please sign in again."))?;
        if req.method != "GET" && !equal(string(&session, "csrf"), &req.csrf) {
            return Err(fail(403, "Invalid security token. Sign in again."));
        }
        Ok(session)
    }
    pub(crate) async fn auth(&mut self, req: &Request, input: Value) -> Result<Response> {
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
                json!({"setup_required":accounts.is_empty(),"authenticated":authenticated,"csrf":session.map(|s|s["csrf"].clone()),"username":accounts.first().filter(|_|authenticated).map(|a|a["username"].clone()),"runtime":self.runtime}),
            ));
        }
        if req.path == "/api/auth/logout" && req.method == "POST" {
            self.authorize(req).await?;
            self.store
                .execute(statement(
                    "DELETE FROM sessions WHERE id=?1",
                    vec![json!(digest(&req.session))],
                ))
                .await?;
            return Ok(Response {
                status: 200,
                body: json!({"ok":true}),
                cookie: Some(cookie("", req.secure, 0)),
            });
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
            let salt = uid();
            let hash = password_hash(password, &salt);
            self.store
                .execute(statement(
                    "INSERT INTO account(id,username,salt,password_hash) VALUES(1,?1,?2,?3)",
                    vec![json!(username), json!(salt), json!(hash)],
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
            let hash = password_hash(password, string(account, "salt"));
            if !equal(username, string(account, "username"))
                || !equal(&hash, string(account, "password_hash"))
            {
                return Err(fail(401, "Incorrect username or password."));
            }
            self.store
                .execute(statement(
                    "DELETE FROM login_attempts WHERE id=?1 OR reset_at<=?2",
                    vec![json!(key), json!(self.now)],
                ))
                .await?;
        }
        let token = format!("{}{}", uid(), uid());
        let csrf = uid();
        self.store
            .batch(vec![
                statement(
                    "DELETE FROM sessions WHERE expires_at<=?1",
                    vec![json!(self.now)],
                ),
                statement(
                    "INSERT INTO sessions(id,csrf,expires_at) VALUES(?1,?2,?3)",
                    vec![
                        json!(digest(&token)),
                        json!(csrf),
                        json!(self.now + SESSION_MS),
                    ],
                ),
            ])
            .await?;
        Ok(Response {
            status: 200,
            body: json!({"authenticated":true,"csrf":csrf,"username":username,"runtime":self.runtime}),
            cookie: Some(cookie(&token, req.secure, SESSION_MS / 1000)),
        })
    }
}
