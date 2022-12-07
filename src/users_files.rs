use crate::{File, sql_struct, User};
use crate::sql_traits::{Insertable, Queryable};
use crate::SQL;
use rocket_db_pools::Connection;
use sqlx::{Error, Mssql, Sqlite};
use sqlx::pool::PoolConnection;
use rocket::serde::Serialize;


sql_struct!(
	Table("UserFiles")
	ID("ID")
	pub struct UserFiles<Sqlite>{
		i32,
		pub UserID:i32,
		pub FileID:i32,
		pub Owner:bool
	}
);

impl Insertable<Fields> for UserFiles{
	fn sql_types_string(&self, field: Fields) -> String {
		match field{
			Fields::id => self.id.to_string(),
			Fields::UserID => self.UserID.to_string(),
			Fields::FileID => self.FileID.to_string(),
			Fields::Owner => match self.Owner{
				true => String::from("1"),
				false => String::from("0"),
			}
		}
	}
}

impl UserFiles{
	pub async fn get_for_user(db: &mut PoolConnection<Sqlite>, user: &User) -> [Vec<File>; 2]{
		let q_owner = format!(r"SELECT F.* FROM UserFiles AS UF JOIN Users AS U ON UF.UserID = U.ID JOIN Files AS F ON F.ID = UF.FileID WHERE U.ID = {}
			AND UF.Owner = 1" ,user.id);
		let q_shared = format!(r"SELECT F.* FROM UserFiles AS UF JOIN Users AS U ON UF.UserID = U.ID JOIN Files AS F ON F.ID = UF.FileID WHERE U.ID = {}
			AND UF.Owner = 0" ,user.id);

		let f0 = sqlx::query_as::<_, File>(&q_owner).fetch_all(&mut *db).await.unwrap();
		let f1 = sqlx::query_as::<_, File>(&q_shared).fetch_all(&mut *db).await.unwrap();
		[f0, f1]
	}

	pub async fn get_file(db: &mut PoolConnection<Sqlite>, user: &User, file_id : i32) ->Option<File>{
		let q = format!("SELECT F.* FROM UserFiles AS UF JOIN Users AS U ON UF.UserID = U.ID JOIN Files AS F ON F.ID = UF.FileID WHERE U.ID = {} AND F.ID = {}",
			user.id, file_id);
		sqlx::query_as::<_, File>(&q)
			.fetch_one(db).await.ok()
	}
}