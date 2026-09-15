use serde_json::Value;
#[derive(Clone, Debug)]
pub struct Statement {
    pub sql: String,
    pub params: Vec<Value>,
}
impl Statement {
    pub fn new(sql: impl Into<String>, params: Vec<Value>) -> Self {
        Self {
            sql: sql.into(),
            params,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Object {
    pub markdown: String,
    pub version: String,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub path: String,
    pub updated_at: u64,
}
#[derive(Clone, Debug, Default)]
pub struct Listing {
    pub entries: Vec<Entry>,
    pub cursor: Option<String>,
}

#[allow(async_fn_in_trait)]
pub trait Store {
    /// Enumerate canonical paths, including hidden workspace records, without reading bodies.
    async fn list(&mut self, cursor: Option<&str>, limit: u32) -> Result<Listing, String>;
    async fn query(&mut self, statement: Statement) -> Result<Vec<Value>, String>;
    async fn execute(&mut self, statement: Statement) -> Result<(), String>;
    /// All statements execute in a SQL transaction.
    async fn batch(&mut self, statements: Vec<Statement>) -> Result<(), String>;
    async fn read(&mut self, path: &str) -> Result<Option<Object>, String>;
    /// None means create-only; Some is compare-and-swap.
    async fn write(
        &mut self,
        path: &str,
        markdown: &str,
        version: Option<&str>,
    ) -> Result<(), String>;
    async fn delete(&mut self, path: &str) -> Result<(), String>;
}

impl<S: Store> Store for &mut S {
    async fn list(&mut self, cursor: Option<&str>, limit: u32) -> Result<Listing, String> {
        (**self).list(cursor, limit).await
    }
    async fn query(&mut self, s: Statement) -> Result<Vec<Value>, String> {
        (**self).query(s).await
    }
    async fn execute(&mut self, s: Statement) -> Result<(), String> {
        (**self).execute(s).await
    }
    async fn batch(&mut self, s: Vec<Statement>) -> Result<(), String> {
        (**self).batch(s).await
    }
    async fn read(&mut self, path: &str) -> Result<Option<Object>, String> {
        (**self).read(path).await
    }
    async fn write(
        &mut self,
        path: &str,
        markdown: &str,
        version: Option<&str>,
    ) -> Result<(), String> {
        (**self).write(path, markdown, version).await
    }
    async fn delete(&mut self, path: &str) -> Result<(), String> {
        (**self).delete(path).await
    }
}
