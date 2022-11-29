use std::collections::HashMap;
use rocket_db_pools::Connection;
use sqlx::pool::PoolConnection;
use sqlx::Sqlite;

pub trait Insertable<E>{
	fn sql_types_string(&self, field: E)->String;
}

#[async_trait]
pub trait Queryable<T: Insertable<E>, I, DB: sqlx::Database, E>{
	fn get_insert_string(&self)->Result<String, String>;
	fn get_update_string(&self)->Result<String, String>;
	fn get_fields(&self)->Vec<String>;
	async fn get_one(db: &mut PoolConnection<DB>,id: I)->Result<T, sqlx::Error>;
	async fn get_all(db: &mut PoolConnection<DB>)->Result<Vec<T>,sqlx::Error>;
	async fn insert(self,db: &mut PoolConnection<DB>)->Result<T,sqlx::Error>;
	async fn update(&self,db: &mut PoolConnection<DB>)->Result<(), sqlx::Error>;
}