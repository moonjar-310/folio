use super::*;
use folio_storage_local::LocalStore;
#[test]
fn folder_rename_resumes_without_losing_canonical_content() {
    futures::executor::block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Application {
            store: LocalStore::open(dir.path().into()).unwrap(),
            now: 1_800_000_000_000,
            setup_secret: None,
            runtime: "local",
        };
        for i in 0..5 {
            app.save_note(
                &format!("rename-{i}"),
                SaveNote {
                    markdown: format!("# Note {i}\n\n- [ ] Preserve task {i}\n"),
                    folder: if i == 0 {
                        "Projects/원본".into()
                    } else {
                        "Projects/원본/Depth/Leaf".into()
                    },
                    revision: None,
                },
            )
            .await
            .unwrap();
        }
        app.mutate_folder("POST", json!({"path":"Projects/원본/Empty/Deeper"}))
            .await
            .unwrap();
        app.mutate_folder("POST", json!({"path":"Projects/원본2"}))
            .await
            .unwrap();
        app.mutate_folder("POST", json!({"path":"Projects/Taken"}))
            .await
            .unwrap();
        assert_eq!(
            app.mutate_folder("PUT", json!({"path":"Projects/원본","name":"Taken"}))
                .await
                .unwrap_err()
                .status,
            409
        );
        assert_eq!(
            app.mutate_folder("PUT", json!({"path":"Projects/원본","name":"../bad"}))
                .await
                .unwrap_err()
                .status,
            400
        );
        let before = app.read_note("rename-0").await.unwrap();
        let tasks = app
            .store
            .query(statement("SELECT id FROM tasks ORDER BY id", vec![]))
            .await
            .unwrap();
        app.store.execute(statement("CREATE TRIGGER interrupt_rename BEFORE UPDATE ON notes WHEN NEW.folder='Projects/Renamed' BEGIN SELECT RAISE(FAIL,'test failure'); END",vec![])).await.unwrap();
        let request = json!({"path":"Projects/원본","name":"Renamed"});
        assert_eq!(
            app.mutate_folder("PUT", request.clone())
                .await
                .unwrap_err()
                .status,
            503
        );
        assert_eq!(
            app.read_note("rename-0").await.unwrap()["folder"],
            "Projects/Renamed"
        );
        assert!(!app.pending_folder_rename().await.unwrap().is_null());
        assert_eq!(
            app.save_note(
                "blocked",
                SaveNote {
                    markdown: "Draft".into(),
                    folder: "Projects/원본".into(),
                    revision: None
                }
            )
            .await
            .unwrap_err()
            .status,
            409
        );
        assert_eq!(
            app.mutate_folder("DELETE", json!({"path":"Projects"}))
                .await
                .unwrap_err()
                .status,
            409
        );
        app.store
            .execute(statement("DROP TRIGGER interrupt_rename", vec![]))
            .await
            .unwrap();
        let mut requests = 0;
        loop {
            requests += 1;
            let result = app.mutate_folder("PUT", request.clone()).await.unwrap();
            if result["done"] == true {
                break;
            }
            assert!(requests < 5);
        }
        assert!(requests >= 3);
        assert!(app.pending_folder_rename().await.unwrap().is_null());
        let after = app.read_note("rename-0").await.unwrap();
        assert_eq!(before["markdown"], after["markdown"]);
        assert_ne!(before["revision"], after["revision"]);
        assert_eq!(
            app.store
                .query(statement("SELECT id FROM tasks ORDER BY id", vec![]))
                .await
                .unwrap(),
            tasks
        );
        let folders = app
            .store
            .query(statement("SELECT path FROM folders ORDER BY path", vec![]))
            .await
            .unwrap();
        assert!(
            folders
                .iter()
                .any(|r| r["path"] == "Projects/Renamed/Empty/Deeper")
        );
        assert!(folders.iter().any(|r| r["path"] == "Projects/원본2"));
        assert!(!folders.iter().any(|r| r["path"] == "Projects/원본"));
        let query = std::collections::HashMap::from([
            ("folder".into(), "Projects/Renamed".into()),
            ("exact".into(), "true".into()),
        ]);
        assert_eq!(
            app.list_notes(&query).await.unwrap()["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            app.save_note(
                "rename-0",
                SaveNote {
                    markdown: "Stale draft".into(),
                    folder: "Projects/원본".into(),
                    revision: Some(string(&before, "revision").into())
                }
            )
            .await
            .unwrap_err()
            .status,
            409
        );
        let raw = app
            .store
            .read("Projects/Renamed/Depth/Leaf/Note 4.md")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(raw.markdown, "# Note 4\n\n- [ ] Preserve task 4\n");
        assert!(!raw.markdown.contains("folio-folder"));
    });
}
#[test]
fn complete_workspace_roundtrip() {
    futures::executor::block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Application {
            store: LocalStore::open(dir.path().into()).unwrap(),
            now: 1_800_000_000_000,
            setup_secret: None,
            runtime: "local",
        };
        let anonymous = app
            .handle(Request {
                method: "GET".into(),
                path: "/api/notes".into(),
                ..Default::default()
            })
            .await;
        assert_eq!(anonymous.status, 401);
        let setup = app
            .handle(Request {
                method: "POST".into(),
                path: "/api/auth/setup".into(),
                body: serde_json::to_vec(
                    &json!({"username":"owner","password":"a long test password"}),
                )
                .unwrap(),
                ..Default::default()
            })
            .await;
        assert_eq!(setup.status, 200, "{:?}", setup.body);
        let cookie = setup.cookie.unwrap();
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        let session = cookie
            .split(';')
            .next()
            .unwrap()
            .strip_prefix("folio_session=")
            .unwrap()
            .to_string();
        let base = Request {
            session,
            csrf: setup.body["csrf"].as_str().unwrap().into(),
            ..Default::default()
        };
        let request = |method: &str, path: &str, body: Value| Request {
            method: method.into(),
            path: path.into(),
            body: serde_json::to_vec(&body).unwrap(),
            ..base.clone()
        };
        let saved=app.handle(request("PUT","/api/notes/n1",json!({"markdown":"# Plan\n\n- [ ] Write 한국어 @due(2026-09-15)\n","folder":"Projects/Folio"}))).await;
        assert_eq!(saved.status, 200, "{:?}", saved.body);
        let tasks = app.handle(request("GET", "/api/tasks", json!({}))).await;
        let task = &tasks.body["items"][0];
        assert_eq!(task["title"], "Write 한국어");
        let task_id = task["id"].as_str().unwrap();
        let toggled = app
            .handle(request(
                "PUT",
                &format!("/api/tasks/{task_id}"),
                json!({"completed":true}),
            ))
            .await;
        assert_eq!(toggled.status, 200, "{:?}", toggled.body);
        let read = app.handle(request("GET", "/api/notes/n1", json!({}))).await;
        assert!(
            read.body["markdown"]
                .as_str()
                .unwrap()
                .contains("- [x] Write 한국어")
        );
        let conflict=app.handle(request("PUT","/api/notes/n1",json!({"markdown":"stale","revision":saved.body["revision"],"folder":"Projects/Folio"}))).await;
        assert_eq!(conflict.status, 409);
        let csrf = app
            .handle(Request {
                csrf: "wrong".into(),
                ..request("POST", "/api/tasks", json!({"title":"no"}))
            })
            .await;
        assert_eq!(csrf.status, 403);
        let origin = app
            .handle(Request {
                origin: Some("https://evil.example".into()),
                expected_origin: "https://folio.example".into(),
                ..request("POST", "/api/tasks", json!({"title":"no"}))
            })
            .await;
        assert_eq!(origin.status, 403);
        let goal = app
            .handle(request(
                "POST",
                "/api/goals",
                json!({"title":"Read more","description":"A quiet hour","position":1}),
            ))
            .await;
        assert_eq!(goal.status, 200);
        let invalid = app
            .handle(request(
                "POST",
                "/api/tasks",
                json!({"title":"bad date","due_date":"2025-02-29"}),
            ))
            .await;
        assert_eq!(invalid.status, 400);
        let before = read.body["markdown"].as_str().unwrap().to_string();
        app.store
            .execute(statement("DROP TABLE folders", vec![]))
            .await
            .unwrap();
        let failed=app.handle(request("PUT","/api/notes/n1",json!({"markdown":format!("{before}\nKept on disk"),"revision":read.body["revision"],"folder":"Projects/Folio"}))).await;
        assert_eq!(failed.status, 503);
        assert!(
            failed.body["message"]
                .as_str()
                .unwrap()
                .starts_with("Markdown saved;")
        );
        assert!(
            app.store
                .read("Projects/Folio/Plan.md")
                .await
                .unwrap()
                .unwrap()
                .markdown
                .contains("Kept on disk")
        );
    });
}

#[test]
fn task_identity_pagination_and_locking() {
    futures::executor::block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Application {
            store: LocalStore::open(dir.path().into()).unwrap(),
            now: 1_800_000_000_000,
            setup_secret: None,
            runtime: "local",
        };
        let saved = app
            .save_note(
                "identity",
                SaveNote {
                    markdown: "# Tasks\n- [ ] Alpha\n- [ ] Beta".into(),
                    folder: "Personal".into(),
                    revision: None,
                },
            )
            .await
            .unwrap();
        let original = app
            .store
            .query(statement(
                "SELECT id,title FROM tasks WHERE source_note_id='identity'",
                vec![],
            ))
            .await
            .unwrap();
        let next = app
            .save_note(
                "identity",
                SaveNote {
                    markdown: "# Tasks\n- [ ] Inserted\n- [ ] Beta\n- [ ] Alpha".into(),
                    folder: "Personal".into(),
                    revision: Some(string(&saved, "revision").into()),
                },
            )
            .await
            .unwrap();
        let after = app
            .store
            .query(statement(
                "SELECT id,title FROM tasks WHERE source_note_id='identity'",
                vec![],
            ))
            .await
            .unwrap();
        for item in original {
            assert!(
                after
                    .iter()
                    .any(|t| t["id"] == item["id"] && t["title"] == item["title"])
            );
        }
        let lock = app.acquire_note("identity").await.unwrap();
        let failure = app
            .save_note(
                "identity",
                SaveNote {
                    markdown: "blocked".into(),
                    folder: "Personal".into(),
                    revision: Some(string(&next, "revision").into()),
                },
            )
            .await
            .unwrap_err();
        assert_eq!(failure.status, 409);
        app.release_note("identity", &lock).await;
        let mut batch = Vec::new();
        for i in 0..60 {
            batch.push(statement("INSERT INTO notes(id,title,preview,updated_at,folder,searchable,revision) VALUES(?1,?2,'preview',?3,'Personal','needle','rev')",vec![json!(format!("page-{i}")),json!(format!("Page {i}")),json!(i+1)]));
        }
        app.store.batch(batch).await.unwrap();
        let first = app
            .list_notes(&[("q".into(), "needle".into())].into())
            .await
            .unwrap();
        assert_eq!(first["items"].as_array().unwrap().len(), 50);
        assert_eq!(first["next_offset"], 50);
        let second = app
            .list_notes(
                &[
                    ("q".into(), "needle".into()),
                    ("offset".into(), "50".into()),
                ]
                .into(),
            )
            .await
            .unwrap();
        assert_eq!(second["items"].as_array().unwrap().len(), 10);
        assert!(second["next_offset"].is_null());
        for task in [
            json!({"title":"Past","due_date":"2026-09-14"}),
            json!({"title":"Future","due_date":"2026-09-20"}),
            json!({"title":"Later"}),
        ] {
            app.save_task(None, task).await.unwrap();
        }
        let today = app
            .list_tasks(
                &[
                    ("group".into(), "today".into()),
                    ("today".into(), "2026-09-15".into()),
                ]
                .into(),
            )
            .await
            .unwrap();
        assert_eq!(today["items"].as_array().unwrap().len(), 1);
        assert_eq!(today["items"][0]["title"], "Past");
        let future = app
            .list_tasks(
                &[
                    ("from".into(), "2026-09-19".into()),
                    ("to".into(), "2026-09-21".into()),
                ]
                .into(),
            )
            .await
            .unwrap();
        assert_eq!(future["items"].as_array().unwrap().len(), 1);
        assert_eq!(future["items"][0]["title"], "Future");
    });
}
#[test]
fn production_setup_is_closed_and_expired_sessions_are_rejected() {
    futures::executor::block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Application {
            store: LocalStore::open(dir.path().into()).unwrap(),
            now: 100_000,
            setup_secret: None,
            runtime: "cloudflare",
        };
        let request = Request {
            method: "POST".into(),
            path: "/api/auth/setup".into(),
            body: serde_json::to_vec(
                &json!({"username":"owner","password":"a secure example password"}),
            )
            .unwrap(),
            secure: true,
            ..Default::default()
        };
        assert_eq!(app.handle(request.clone()).await.status, 503);
        app.setup_secret = Some("a-long-initial-setup-token-for-tests".into());
        assert_eq!(app.handle(request).await.status, 403);
        app.store
            .execute(statement(
                "INSERT INTO sessions(id,csrf,expires_at) VALUES(?1,'csrf',1)",
                vec![json!(digest("expired"))],
            ))
            .await
            .unwrap();
        assert_eq!(
            app.handle(Request {
                method: "GET".into(),
                path: "/api/notes".into(),
                session: "expired".into(),
                ..Default::default()
            })
            .await
            .status,
            401
        );
        app.store
            .execute(statement(
                "INSERT INTO login_attempts(id,count,reset_at) VALUES(?1,11,?2)",
                vec![json!(digest("blocked")), json!(200_000)],
            ))
            .await
            .unwrap();
        assert_eq!(
            app.handle(Request {
                method: "POST".into(),
                path: "/api/auth/login".into(),
                client: "blocked".into(),
                body: serde_json::to_vec(&json!({"username":"owner","password":"wrong"})).unwrap(),
                ..Default::default()
            })
            .await
            .status,
            429
        );
    });
}

#[test]
fn paths_survive_loss_of_indexes_and_all_auxiliary_note_metadata() {
    futures::executor::block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Application {
            store: LocalStore::open(dir.path().into()).unwrap(),
            now: 1_800_000_000_000,
            setup_secret: None,
            runtime: "local",
        };
        app.save_note(
            "portable",
            SaveNote {
                markdown: "# 설계\n\n- [ ] Original task\n".into(),
                folder: "Projects/Folio".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
        let task = app
            .save_task(None, json!({"title":"Independent","due_date":"2026-09-15"}))
            .await
            .unwrap();
        let goal = app
            .save_goal(None, json!({"title":"Direction"}))
            .await
            .unwrap();
        assert_eq!(
            app.store
                .read("Projects/Folio/설계.md")
                .await
                .unwrap()
                .unwrap()
                .markdown,
            "# 설계\n\n- [ ] Original task\n"
        );
        app.store
            .batch(vec![
                statement("DELETE FROM notes", vec![]),
                statement("DELETE FROM tasks", vec![]),
                statement("DELETE FROM goals", vec![]),
                statement("DELETE FROM folders", vec![]),
            ])
            .await
            .unwrap();
        // A cacheless listing and open work BEFORE any reconstruction.
        let listing = app.list_notes(&Default::default()).await.unwrap();
        assert_eq!(listing["items"].as_array().unwrap().len(), 1);
        let id = string(&listing["items"][0], "id").to_string();
        assert_eq!(
            app.read_note(&id).await.unwrap()["folder"],
            "Projects/Folio"
        );
        let mut input = json!({});
        for _ in 0..100 {
            let result = app.rebuild(&input).await.unwrap();
            if result["done"] == true {
                break;
            }
            input = result;
        }
        assert_eq!(
            app.store
                .query(statement(
                    "SELECT title FROM tasks WHERE id=?1",
                    vec![task["id"].clone()]
                ))
                .await
                .unwrap()[0]["title"],
            "Independent"
        );
        assert_eq!(
            app.store
                .query(statement(
                    "SELECT title FROM goals WHERE id=?1",
                    vec![goal["id"].clone()]
                ))
                .await
                .unwrap()[0]["title"],
            "Direction"
        );
        assert_eq!(
            app.list_notes(&[("q".into(), "Original task".into())].into())
                .await
                .unwrap()["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        // Lose the optional identity record while the SQL cache remains.
        app.store
            .delete(&format!(".folio/notes/{}.json", digest("portable")))
            .await
            .unwrap();
        assert!(app.read_note("portable").await.is_ok());
        app.store
            .batch(vec![
                statement("DELETE FROM notes", vec![]),
                statement("DELETE FROM tasks WHERE source_note_id IS NOT NULL", vec![]),
                statement("DELETE FROM folders", vec![]),
            ])
            .await
            .unwrap();
        let listing = app.list_notes(&Default::default()).await.unwrap();
        let id = string(&listing["items"][0], "id").to_string();
        let note: Note = serde_json::from_value(app.read_note(&id).await.unwrap()).unwrap();
        app.save_note(
            &id,
            SaveNote {
                markdown: format!("{}\nEdited after loss", note.markdown),
                folder: note.folder,
                revision: Some(note.revision),
            },
        )
        .await
        .unwrap();
        assert!(
            app.store
                .read("Projects/Folio/설계.md")
                .await
                .unwrap()
                .unwrap()
                .markdown
                .ends_with("Edited after loss")
        );
        // An arbitrary file with no Folio header or sidecar is discoverable and editable.
        app.store
            .write("External.md", "Plain external Markdown", None)
            .await
            .unwrap();
        let ext = paths::path_id("External.md");
        let note: Note = serde_json::from_value(app.read_note(&ext).await.unwrap()).unwrap();
        assert_eq!(note.folder, "");
        app.save_note(
            &ext,
            SaveNote {
                markdown: "# External\nEdited".into(),
                folder: String::new(),
                revision: Some(note.revision),
            },
        )
        .await
        .unwrap();
        assert!(
            app.store
                .read("External.md")
                .await
                .unwrap()
                .unwrap()
                .markdown
                .contains("Edited")
        );
    });
}
#[test]
fn legacy_migration_preserves_original_and_retries_without_duplicate_files() {
    futures::executor::block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Application {
            store: LocalStore::open(dir.path().into()).unwrap(),
            now: 10,
            setup_secret: None,
            runtime: "local",
        };
        let original = "<!-- folio-folder:\"Projects/Legacy\" -->\n# Old title\n\nKeep exactly.\n";
        app.store.write("legacy.md", original, None).await.unwrap();
        app.store.execute(statement("INSERT INTO notes(id,title,preview,updated_at,folder,revision) VALUES('legacy','Old title','',1,'Projects/Legacy','')",vec![])).await.unwrap();
        app.store
            .execute(statement(
                "INSERT INTO folders(path) VALUES('Empty/Nested')",
                vec![],
            ))
            .await
            .unwrap();
        let mut input = json!({});
        let mut done = false;
        for _ in 0..50 {
            let next = app.migrate(&input).await.unwrap();
            if next["done"] == true {
                done = true;
                break;
            }
            input = next;
        }
        assert!(done);
        assert!(app.store.read("legacy.md").await.unwrap().is_none());
        assert_eq!(
            app.store
                .read(".folio/legacy/legacy.md")
                .await
                .unwrap()
                .unwrap()
                .markdown,
            original
        );
        assert_eq!(
            app.store
                .read("Projects/Legacy/Old title.md")
                .await
                .unwrap()
                .unwrap()
                .markdown,
            "# Old title\n\nKeep exactly.\n"
        );
        assert_eq!(
            app.read_note("legacy").await.unwrap()["folder"],
            "Projects/Legacy"
        );
        assert!(
            app.store
                .read("Empty/Nested/.folio-folder")
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(
            app.list_notes(&Default::default()).await.unwrap()["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        // Two old IDs can share a title; retain both originals without changing their bodies.
        app.store
            .write("duplicate.md", original, None)
            .await
            .unwrap();
        app.store.execute(statement("INSERT INTO notes(id,title,preview,updated_at,folder,revision) VALUES('duplicate','Old title','',1,'Projects/Legacy','')",vec![])).await.unwrap();
        app.migrate(&json!({})).await.unwrap();
        let files = app.list_notes(&Default::default()).await.unwrap();
        assert_eq!(files["items"].as_array().unwrap().len(), 2);
        assert_eq!(
            app.read_note("duplicate").await.unwrap()["markdown"],
            "# Old title\n\nKeep exactly.\n"
        );
        assert!(app.migrate(&json!({})).await.is_ok());
    });
}
#[test]
fn duplicate_names_and_traversal_never_overwrite_files() {
    futures::executor::block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Application {
            store: LocalStore::open(dir.path().into()).unwrap(),
            now: 10,
            setup_secret: None,
            runtime: "local",
        };
        app.save_note(
            "one",
            SaveNote {
                markdown: "# Same\nFirst".into(),
                folder: "Personal".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
        let conflict = app
            .save_note(
                "two",
                SaveNote {
                    markdown: "# Same\nSecond".into(),
                    folder: "Personal".into(),
                    revision: None,
                },
            )
            .await
            .unwrap_err();
        assert_eq!(conflict.status, 409);
        assert_eq!(
            app.store
                .read("Personal/Same.md")
                .await
                .unwrap()
                .unwrap()
                .markdown,
            "# Same\nFirst"
        );
        assert_eq!(
            app.save_note(
                "case",
                SaveNote {
                    markdown: "# same\nDifferent case".into(),
                    folder: "Personal".into(),
                    revision: None
                }
            )
            .await
            .unwrap_err()
            .status,
            409
        );
        assert_eq!(
            app.mutate_folder("POST", json!({"path":"personal"}))
                .await
                .unwrap_err()
                .status,
            409
        );
        assert_eq!(
            app.mutate_folder("PUT", json!({"path":"Personal","name":"personal"}))
                .await
                .unwrap_err()
                .status,
            409
        );
        assert!(app.pending_folder_rename().await.unwrap().is_null());
        for path in [
            "../outside.md",
            "/outside.md",
            "C:/outside.md",
            "a/../../outside.md",
            "a/CON.md",
            "a./b.md",
            "a\\b.md",
        ] {
            assert!(app.store.write(path, "bad", None).await.is_err(), "{path}");
        }
        for folder in ["../a", ".folio", "A/../B", "A/NUL", "A."] {
            assert!(!valid_folder(folder));
        }
    });
}

struct InterruptedStore {
    inner: LocalStore,
    fail_delete: Option<String>,
}
impl Store for InterruptedStore {
    async fn list(
        &mut self,
        c: Option<&str>,
        n: u32,
    ) -> std::result::Result<folio_core::storage::Listing, String> {
        self.inner.list(c, n).await
    }
    async fn query(&mut self, s: Statement) -> std::result::Result<Vec<Value>, String> {
        self.inner.query(s).await
    }
    async fn execute(&mut self, s: Statement) -> std::result::Result<(), String> {
        self.inner.execute(s).await
    }
    async fn batch(&mut self, s: Vec<Statement>) -> std::result::Result<(), String> {
        self.inner.batch(s).await
    }
    async fn read(
        &mut self,
        p: &str,
    ) -> std::result::Result<Option<folio_core::storage::Object>, String> {
        self.inner.read(p).await
    }
    async fn write(
        &mut self,
        p: &str,
        b: &str,
        v: Option<&str>,
    ) -> std::result::Result<(), String> {
        self.inner.write(p, b, v).await
    }
    async fn delete(&mut self, p: &str) -> std::result::Result<(), String> {
        if self.fail_delete.as_deref() == Some(p) {
            self.fail_delete = None;
            Err("Injected object-delete interruption".into())
        } else {
            self.inner.delete(p).await
        }
    }
}
#[test]
fn interrupted_copy_delete_move_resumes_and_preserves_source_edits() {
    futures::executor::block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Application {
            store: InterruptedStore {
                inner: LocalStore::open(dir.path().into()).unwrap(),
                fail_delete: None,
            },
            now: 10,
            setup_secret: None,
            runtime: "local",
        };
        let saved = app
            .save_note(
                "move",
                SaveNote {
                    markdown: "# Move me\nBody".into(),
                    folder: "Before".into(),
                    revision: None,
                },
            )
            .await
            .unwrap();
        app.store.fail_delete = Some("Before/Move me.md".into());
        let input = SaveNote {
            markdown: "# Move me\nBody".into(),
            folder: "After".into(),
            revision: Some(string(&saved, "revision").into()),
        };
        assert_eq!(
            app.save_note("move", input.clone())
                .await
                .unwrap_err()
                .status,
            503
        );
        assert!(app.store.read("Before/Move me.md").await.unwrap().is_some());
        assert!(app.store.read("After/Move me.md").await.unwrap().is_some());
        app.save_note("move", input).await.unwrap();
        assert!(app.store.read("Before/Move me.md").await.unwrap().is_none());
        assert_eq!(
            app.read_note("move").await.unwrap()["markdown"],
            "# Move me\nBody"
        );
        let saved = app.read_note("move").await.unwrap();
        app.store.fail_delete = Some("After/Move me.md".into());
        let input = SaveNote {
            markdown: "# Move me\nBody".into(),
            folder: "Last".into(),
            revision: Some(string(&saved, "revision").into()),
        };
        assert!(app.save_note("move", input.clone()).await.is_err());
        let old = app.store.read("After/Move me.md").await.unwrap().unwrap();
        app.store
            .write(
                "After/Move me.md",
                "External edit must survive",
                Some(&old.version),
            )
            .await
            .unwrap();
        assert_eq!(app.save_note("move", input).await.unwrap_err().status, 409);
        assert_eq!(
            app.store
                .read("After/Move me.md")
                .await
                .unwrap()
                .unwrap()
                .markdown,
            "External edit must survive"
        );
    });
}
#[test]
fn rebuild_skips_bad_auxiliary_records_and_removes_deleted_search_results() {
    futures::executor::block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Application {
            store: LocalStore::open(dir.path().into()).unwrap(),
            now: 10,
            setup_secret: None,
            runtime: "local",
        };
        app.store
            .write(".folio/notes/bad.json", "not json", None)
            .await
            .unwrap();
        app.store
            .write("Imported/Standalone.md", "# Standalone\nFindable", None)
            .await
            .unwrap();
        let mut warnings = 0;
        let mut input = json!({});
        let mut done = false;
        for _ in 0..100 {
            let next = app.rebuild(&input).await.unwrap();
            warnings += next["warnings"].as_array().map(Vec::len).unwrap_or(0);
            if next["done"] == true {
                done = true;
                break;
            }
            input = next;
        }
        assert!(done);
        assert_eq!(warnings, 1);
        assert_eq!(
            app.list_notes(&[("q".into(), "Findable".into())].into())
                .await
                .unwrap()["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        app.store.delete("Imported/Standalone.md").await.unwrap();
        let mut input = json!({});
        let mut done = false;
        for _ in 0..100 {
            let next = app.rebuild(&input).await.unwrap();
            if next["done"] == true {
                done = true;
                break;
            }
            input = next;
        }
        assert!(done);
        assert!(
            app.list_notes(&[("q".into(), "Findable".into())].into())
                .await
                .unwrap()["items"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    });
}
