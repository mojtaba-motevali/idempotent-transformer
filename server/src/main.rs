mod database;
mod rpc_server;
mod schema;

use database::db::{get_connection, init_tables};
use libsql::{Error, params};
use rpc_server::server::start_server;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let conn = get_connection().await?;
    init_tables(&conn).await?;

    let mut stmt = conn.prepare("SELECT * FROM workflows").await.unwrap();
    let mut rows = stmt.query(params![]).await?;
    while let Some(row) = rows.next().await? {
        println!("{:?}", row.get::<String>(0));
    }

    start_server(conn).await.unwrap();

    Ok(())
}
