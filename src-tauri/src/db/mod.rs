//! SQLite storage: connection setup, migrations and queries.

pub mod migrations;
pub mod queries;
pub mod schema;

use rusqlite::Connection;
use std::path::Path;

pub fn configure(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA temp_store = MEMORY;
         PRAGMA busy_timeout = 3000;",
    )
}

pub fn open(path: &Path) -> anyhow::Result<Connection> {
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    migrations::run(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_memory() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    configure(&conn).unwrap();
    migrations::run(&mut conn).unwrap();
    conn
}
