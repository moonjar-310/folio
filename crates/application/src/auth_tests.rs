use super::*;
use auth::{AuthMethod, VerifiedIdentity};
use folio_storage_local::LocalStore;

fn request(path: &str, password: &str) -> Request {
    Request {
        method: "POST".into(),
        path: path.into(),
        body: serde_json::to_vec(&json!({"username":"owner","password":password})).unwrap(),
        ..Default::default()
    }
}

#[test]
fn password_setup_uses_argon2id_and_legacy_upgrade_requires_correct_password() {
    futures::executor::block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Application {
            store: LocalStore::open(dir.path().into()).unwrap(),
            now: 1_800_000_000_000,
            setup_secret: None,
            jwt_secret: "test-key-only-not-for-production!".repeat(3),
            runtime: "local",
            auth_method: AuthMethod::Password,
        };
        let password = "A long local password!";
        assert_eq!(
            app.handle(request("/api/auth/setup", password))
                .await
                .status,
            200
        );
        let hash = app
            .store
            .query(statement("SELECT password_hash,salt FROM account", vec![]))
            .await
            .unwrap();
        assert!(string(&hash[0], "password_hash").starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
        assert_eq!(
            crate::password::verify(password, "", string(&hash[0], "password_hash")),
            Some(false)
        );
        assert_eq!(
            app.handle(request("/api/auth/login", "wrong")).await.status,
            401
        );

        let mut legacy = [0u8; 32];
        pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), b"old-salt", 600_000, &mut legacy);
        let legacy: String = legacy.iter().map(|b| format!("{b:02x}")).collect();
        app.store
            .execute(statement(
                "UPDATE account SET salt='old-salt',password_hash=?1",
                vec![json!(legacy)],
            ))
            .await
            .unwrap();
        assert_eq!(
            app.handle(request("/api/auth/login", "wrong")).await.status,
            401
        );
        assert_eq!(
            app.store
                .query(statement("SELECT password_hash FROM account", vec![]))
                .await
                .unwrap()[0]["password_hash"],
            legacy
        );
        let login = app.handle(request("/api/auth/login", password)).await;
        assert_eq!(login.status, 200);
        assert!(login.cookie.unwrap().contains("HttpOnly; SameSite=Strict"));
        let upgraded = app
            .store
            .query(statement("SELECT password_hash,salt FROM account", vec![]))
            .await
            .unwrap();
        assert_eq!(upgraded[0]["salt"], "");
        assert!(string(&upgraded[0], "password_hash").starts_with("$argon2id$"));
        assert_eq!(
            app.handle(request("/api/auth/login", password))
                .await
                .status,
            200
        );
    });
}

#[test]
fn access_and_refresh_lifetimes_rotation_and_no_session_reads() {
    futures::executor::block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Application {
            store: LocalStore::open(dir.path().into()).unwrap(),
            now: 1_800_000_000_000,
            setup_secret: None,
            jwt_secret: "test-key-only-not-for-production!".repeat(3),
            runtime: "cloudflare",
            auth_method: AuthMethod::CloudflareAccess,
        };
        let identity = VerifiedIdentity {
            subject: "https://team.cloudflareaccess.com|owner".into(),
            username: "owner@folio.dev".into(),
            expires_at: app.now + 8 * 86_400_000,
        };
        let mut req = Request {
            method: "POST".into(),
            path: "/api/auth/login".into(),
            identity: Some(identity),
            secure: true,
            origin: Some("https://folio.test".into()),
            expected_origin: "https://folio.test".into(),
            ..Default::default()
        };
        let auth = app.handle(req.clone()).await;
        assert_eq!(auth.status, 200);
        assert_eq!(auth.body["expires_in"], 300);
        let cookie = auth.cookie.unwrap();
        assert!(cookie.starts_with("__Host-folio_refresh="));
        assert!(cookie.contains("Max-Age=604800; Secure"));
        let refresh = cookie
            .split(';')
            .next()
            .unwrap()
            .split_once('=')
            .unwrap()
            .1
            .to_string();
        req.session = refresh.clone();
        req.access_token = string(&auth.body, "access_token").into();
        req.csrf = string(&auth.body, "csrf").into();
        let row = app
            .store
            .query(statement("SELECT * FROM sessions", vec![]))
            .await
            .unwrap();
        assert_eq!(row[0]["id"], digest(&refresh));
        assert_eq!(row[0]["expires_at"], app.now + 604_800_000);
        req.method = "GET".into();
        assert!(app.authorize(&req).await.is_ok());
        let mut bad = req.clone();
        bad.access_token.push('x');
        assert_eq!(app.authorize(&bad).await.unwrap_err().status, 401);
        let mut bad = req.clone();
        bad.identity.as_mut().unwrap().subject = "other".into();
        assert_eq!(app.authorize(&bad).await.unwrap_err().status, 401);
        let mut bad = req.clone();
        bad.method = "PUT".into();
        bad.csrf = "forged".into();
        assert_eq!(app.authorize(&bad).await.unwrap_err().status, 403);
        app.now += 299_000;
        assert!(app.authorize(&req).await.is_ok());
        app.now += 1000;
        assert_eq!(app.authorize(&req).await.unwrap_err().status, 401);
        req.path = "/api/auth/refresh".into();
        req.method = "POST".into();
        let mut bad = req.clone();
        bad.origin = None;
        assert_eq!(app.handle(bad).await.status, 403);
        let rotated = app.handle(req.clone()).await;
        assert_eq!(rotated.status, 200);
        assert_eq!(
            app.handle(req.clone()).await.status,
            401,
            "old refresh cannot be replayed"
        );
        req.session = rotated
            .cookie
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .split_once('=')
            .unwrap()
            .1
            .into();
        req.access_token = string(&rotated.body, "access_token").into();
        // Authorization still works when the entire refresh table is inaccessible.
        app.store
            .execute(statement(
                "ALTER TABLE sessions RENAME TO hidden_sessions",
                vec![],
            ))
            .await
            .unwrap();
        assert!(app.authorize(&req).await.is_ok());
        app.store
            .execute(statement(
                "ALTER TABLE hidden_sessions RENAME TO sessions",
                vec![],
            ))
            .await
            .unwrap();
        app.now = 1_800_000_000_000 + 604_800_000;
        assert_eq!(
            app.handle(req).await.status,
            401,
            "rotation does not extend the one-week lifetime"
        );
    });
}
