use std::fmt::{Display, Formatter};
use rocket::http::CookieJar;
use crate::{sql_struct, User};
use crate::sql_traits::{Insertable, Queryable};
use crate::SQL;
use rocket_db_pools::Connection;
use sqlx::{Error, Sqlite};
use sqlx::pool::PoolConnection;
use rocket::serde::Serialize;

sql_struct!(
	Table("Files")
	ID("ID")
	pub struct File<Sqlite>{
		i32,
		pub UserID:i32,
		pub Filename:String,
		pub Content:String,
		pub MimeType:Option<String>,
	}
);

impl Display for File
{
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", &self.Filename)
	}
}

impl Insertable for File {
	fn get_insert_string(&self) -> String {
		format!(r"INSERT INTO Files (UserID, Filename, Content, MimeType) VALUES ({},'{}','{}', {}) RETURNING *",
				&self.UserID, &self.Filename, &self.Content,
				match &self.MimeType {
					None => "null".to_string(),
					Some(t) => format!("'{}'", t)
				}
		)
	}

	fn get_update_string(&self) -> String {
		format!(r"UPDATE Files SET UserID={}, Filename='{}', Content='{}', MimeType={} WHERE ID = {} RETURNING *",
				&self.UserID, &self.Filename, &self.Content,
				match &self.MimeType {
					None => "null".to_string(),
					Some(t) => format!("'{}'", t)
				}, &self.id
		)
	}
}

impl File {
	pub async fn get_for_user(db: &mut PoolConnection<Sqlite>, user: &User) -> Vec<File> {
		sqlx::query_as::<_, File>(&format!("SELECT F.* FROM Files AS F JOIN Users AS U ON F.UserID=U.ID WHERE U.ID={}", &user.id))
			.fetch_all(db).await.ok().unwrap()
	}

	pub async fn delete_file_from_user(self,db: &mut PoolConnection<Sqlite>, jar:&CookieJar<'_>)->Result<(),String>{
		match User::get_from_cookies(db,jar).await{
			None => Err(String::from("Należy się zalogować")),
			Some(u) => {
				if u.id==self.UserID{
					match sqlx::query_as::<_,File>(&format!("DELETE FROM Files WHERE ID={} AND UserID={}",self.id, self.UserID)).fetch_all(db).await {
						Ok(_) => Ok(()),
						Err(e) => Err(format!("{:?}",e))
					}
				}else{
					Err(String::from("Nie masz dostępu do tego pliku"))
				}
			}
		}
	}

	pub async fn change_filename(&mut self,db: &mut PoolConnection<Sqlite>, jar:&CookieJar<'_>, new_filename:String)->Result<(), &'static str>{
		match User::get_from_cookies(db, jar).await{
			None => Err("Należy się zalogować"),
			Some(user) => {
				match user.id==self.UserID{
					true => {
						self.Filename=new_filename;
						&self.update(db).await;
						Ok(())
					}
					false => Err("Nie masz dostępu do tego pliku")
				}
			}
		}
	}
}