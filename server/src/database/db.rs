use libsql::{Builder, Connection, Error, params};

pub async fn get_connection() -> Result<Connection, Error> {
    let db = Builder::new_local("local.db").build().await.unwrap();
    let conn = db.connect().unwrap();
    Ok(conn)
}

pub async fn init_tables(conn: &Connection) -> Result<(), Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS workflows (id TEXT PRIMARY KEY, execution_order INTEGER NOT NULL, value BLOB NOT NULL, expire_at TIMESTAMP)",
        params![],
    )
    .await?;
    Ok(())
}
