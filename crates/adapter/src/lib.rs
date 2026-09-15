//! Runtime composition: select the implementation through the entrypoint's Cargo feature.
#[cfg(feature = "cloudflare")]
pub use folio_storage_cloudflare::CloudflareStore;
#[cfg(feature = "local")]
pub use folio_storage_local::LocalStore;
#[cfg(all(feature = "local", not(target_arch = "wasm32")))]
pub type RuntimeStore = LocalStore;
#[cfg(all(feature = "cloudflare", target_arch = "wasm32"))]
pub type RuntimeStore = CloudflareStore;
