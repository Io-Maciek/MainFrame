use rocket_db_pools::Connection;
use sqlx::pool::PoolConnection;
use sqlx::Sqlite;

pub trait Insertable{
	fn get_insert_string(&self)->String;
	fn get_update_string(&self)->String;
}

#[async_trait]
pub trait Queryable<T: Insertable, I, DB: sqlx::Database>{
	async fn get_one(db: &mut PoolConnection<DB>,id: I)->Result<T, sqlx::Error>;
	async fn get_all(db: &mut PoolConnection<DB>)->Result<Vec<T>,sqlx::Error>;
	async fn insert(self,db: &mut PoolConnection<DB>)->Result<T,sqlx::Error>;
	async fn update(&self,db: &mut PoolConnection<DB>)->Result<(), sqlx::Error>;
}