//! `fluxo-server` binary — serves the Fluxo HTTP API backed by a SQLite store.
//!
//! Configuration via environment:
//! - `FLUXO_ADDR` (default `127.0.0.1:8080`)
//! - `FLUXO_DB`   (default `fluxo.db`; use `:memory:` for an ephemeral store)

use fluxo_engine::Engine;
use fluxo_store::sqlite::SqliteStore;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = std::env::var("FLUXO_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()?;
    let db = std::env::var("FLUXO_DB").unwrap_or_else(|_| "fluxo.db".to_string());

    let store = if db == ":memory:" {
        SqliteStore::open_in_memory()?
    } else {
        SqliteStore::open(&db)?
    };
    let engine = Engine::new(store);

    println!("fluxo-server listening on http://{addr}  (db: {db})");
    fluxo_server::serve(engine, addr).await?;
    Ok(())
}
