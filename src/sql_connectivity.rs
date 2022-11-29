use rocket_db_pools::{sqlx, Database};

#[derive(Database)]
#[database("MainFrame")]
pub struct SQL(pub sqlx::SqlitePool);
//pub struct SQL(pub sqlx::MssqlPool);