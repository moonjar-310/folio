//! Cloudflare-specific APIs never escape this infrastructure implementation.
use folio_core::{
    paths::valid_path,
    storage::{Entry, Listing, Object, Statement, Store},
};
use serde_json::Value;
use worker::{Bucket, Conditional, D1Database, D1PreparedStatement, Env, wasm_bindgen::JsValue};
mod timing;
pub use timing::Timings;
pub struct CloudflareStore {
    db: D1Database,
    bucket: Bucket,
    timings: Timings,
}
impl CloudflareStore {
    pub fn new(env: &Env) -> Result<Self, String> {
        Ok(Self {
            db: env.d1("DB").map_err(|e| e.to_string())?,
            bucket: env.bucket("NOTES").map_err(|e| e.to_string())?,
            timings: Timings::default(),
        })
    }
    pub fn timings(&self) -> Timings {
        self.timings.clone()
    }
    fn prepare(&self, s: Statement) -> Result<D1PreparedStatement, String> {
        let values: Vec<JsValue> = s
            .params
            .into_iter()
            .map(|v| match v {
                Value::Null => JsValue::NULL,
                Value::Bool(b) => JsValue::from_f64(if b { 1.0 } else { 0.0 }),
                Value::Number(n) => JsValue::from_f64(n.as_f64().unwrap_or(0.0)),
                Value::String(s) => JsValue::from_str(&s),
                _ => JsValue::from_str(&v.to_string()),
            })
            .collect();
        self.db
            .prepare(s.sql)
            .bind(&values)
            .map_err(|e| e.to_string())
    }
}
fn key(path: &str) -> Result<String, String> {
    if !valid_path(path) {
        return Err("Invalid vault path".into());
    }
    Ok(format!("notes/{path}"))
}
impl Store for CloudflareStore {
    async fn list(&mut self, cursor: Option<&str>, limit: u32) -> Result<Listing, String> {
        let _span = self.timings.start("r2_list");
        let mut builder = self
            .bucket
            .list()
            .prefix("notes/")
            .limit(limit.clamp(1, 1000));
        if let Some(cursor) = cursor {
            builder = builder.cursor(cursor);
        }
        let result = builder.execute().await.map_err(|e| e.to_string())?;
        Ok(Listing {
            entries: result
                .objects()
                .into_iter()
                .map(|o| Entry {
                    path: o.key().trim_start_matches("notes/").into(),
                    updated_at: o.uploaded().as_millis(),
                })
                .collect(),
            cursor: result.cursor(),
        })
    }

    async fn query(&mut self, s: Statement) -> Result<Vec<Value>, String> {
        let _span = self.timings.start("d1_query");
        self.prepare(s)?
            .all()
            .await
            .map_err(|e| e.to_string())?
            .results::<Value>()
            .map_err(|e| e.to_string())
    }
    async fn execute(&mut self, s: Statement) -> Result<(), String> {
        let _span = self.timings.start("d1_execute");
        let result = self.prepare(s)?.run().await.map_err(|e| e.to_string())?;
        if result.success() {
            Ok(())
        } else {
            Err("D1 statement failed".into())
        }
    }
    async fn batch(&mut self, statements: Vec<Statement>) -> Result<(), String> {
        let _span = self.timings.start("d1_batch");
        let statements = statements
            .into_iter()
            .map(|s| self.prepare(s))
            .collect::<Result<Vec<_>, _>>()?;
        let results = self.db.batch(statements).await.map_err(|e| e.to_string())?;
        if results.iter().all(|r| r.success()) {
            Ok(())
        } else {
            Err("D1 transaction failed".into())
        }
    }
    async fn read(&mut self, id: &str) -> Result<Option<Object>, String> {
        let _span = self.timings.start("r2_read");
        let Some(object) = self
            .bucket
            .get(key(id)?)
            .execute()
            .await
            .map_err(|e| e.to_string())?
        else {
            return Ok(None);
        };
        let version = object.etag();
        let markdown = object
            .body()
            .ok_or("Missing R2 body")?
            .text()
            .await
            .map_err(|e| e.to_string())?;
        Ok(Some(Object { markdown, version }))
    }
    async fn write(
        &mut self,
        id: &str,
        markdown: &str,
        version: Option<&str>,
    ) -> Result<(), String> {
        let _span = self.timings.start("r2_write");
        let condition = match version {
            Some(v) => Conditional {
                etag_matches: Some(v.into()),
                ..Default::default()
            },
            None => Conditional {
                etag_does_not_match: Some("*".into()),
                ..Default::default()
            },
        };
        self.bucket
            .put(key(id)?, markdown.to_string())
            .only_if(condition)
            .execute()
            .await
            .map_err(|e| e.to_string())?
            .map(|_| ())
            .ok_or("conflict".into())
    }
    async fn delete(&mut self, id: &str) -> Result<(), String> {
        let _span = self.timings.start("r2_delete");
        self.bucket
            .delete(key(id)?)
            .await
            .map_err(|e| e.to_string())
    }
}
