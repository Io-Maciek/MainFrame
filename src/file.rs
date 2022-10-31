use std::fmt::{Display, Formatter};
use crate::{create, User};
use crate::sql_traits::{Insertable, Queryable};
use crate::SQL;
use rocket_db_pools::Connection;
use sqlx::{Error, Sqlite};
use sqlx::pool::PoolConnection;

create!(
	#[Table("Files")]
	pub struct File<Sqlite>{
		i32,
		pub UserID:i32,
		pub Filename:String,
		pub Content:String
	}
);

impl Display for File
{
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f,"{} {} {}", &self.ID,&self.UserID, &self.Filename)
	}
}

impl Insertable for File {
	fn get_insert_string(&self) -> String {
		format!(r"INSERT INTO Files (UserID, Filename, Content) VALUES ({},'{}','{}') RETURNING *",
				&self.UserID, &self.Filename, &self.Content)
	}

	fn get_update_string(&self) -> String {
		format!(r"UPDATE Files SET UserID={}, Filename='{}', Content='{}' WHERE ID = {} RETURNING *",
				&self.UserID, &self.Filename, &self.Content, &self.ID)
	}
}

impl File{
	pub async fn get_for_user(db: &mut PoolConnection<Sqlite>,user: &User,)->Vec<File>{
		sqlx::query_as::<_, File>(&format!("SELECT F.* FROM Files AS F JOIN Users AS U ON F.UserID=U.ID WHERE U.ID={}",&user.ID))
			.fetch_all(db).await.ok().unwrap()
	}
}