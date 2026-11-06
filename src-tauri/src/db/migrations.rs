//! Versioned migrations driven by `PRAGMA user_version`.

use super::schema::MIGRATIONS;
use rusqlite::Connection;

pub fn run(conn: &mut Connection) -> anyhow::Result<()> {
    let current: usize =
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? as usize;
    for (idx, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (idx + 1) as i64)?;
        tx.commit()?;
        log::info!("database migrated to version {}", idx + 1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent() {
        let mut c = Connection::open_in_memory().unwrap();
        run(&mut c).unwrap();
        run(&mut c).unwrap();
        let v: i64 = c
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v as usize, MIGRATIONS.len());
        let spaces: i64 = c
            .query_row("SELECT count(*) FROM spaces", [], |r| r.get(0))
            .unwrap();
        assert_eq!(spaces, 1);
    }
}
