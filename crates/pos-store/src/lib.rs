//! `pos-store` — SQLite persistence via `rusqlite` (bundled) with embedded,
//! versioned migrations.
//!
//! Migrations are embedded with `include_str!` so the single-binary promise
//! holds (no runtime `migrations/` directory is required).

pub mod auth;
pub mod error;
pub mod inventory;
pub mod products;
pub mod reports;
pub mod sales;
pub mod settings;

use std::path::Path;

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

pub use error::{Result, StoreError};

/// The database handle.
pub struct Db {
    conn: Connection,
}

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!(
        "../../../migrations/0001_init.sql"
    ))])
}

impl Db {
    /// Open (creating if needed) a database file and run migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    /// An in-memory database, used by tests.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    fn from_connection(mut conn: Connection) -> Result<Self> {
        // Crash-safety and sane concurrency for a single-terminal app.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;

        migrations().to_latest(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Back up the database to `dest` using SQLite's online backup API.
    pub fn backup_to(&self, dest: impl AsRef<Path>) -> Result<()> {
        let mut out = Connection::open(dest)?;
        let backup = rusqlite::backup::Backup::new(&self.conn, &mut out)?;
        backup.run_to_completion(64, std::time::Duration::from_millis(5), None)?;
        Ok(())
    }
}

/// Read the schema version (SQLite `user_version`).
impl Db {
    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }
}
