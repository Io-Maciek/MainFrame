use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use rocket::http::CookieJar;
use crate::{sql_struct, User};
use crate::sql_traits::{Insertable, Queryable};
use crate::SQL;
use rocket_db_pools::Connection;
use sqlx::{Error, Mssql, Sqlite};
use sqlx::pool::PoolConnection;
use rocket::serde::Serialize;
use serde_json::error::Category::Data;
use crate::users_files::UserFiles;

sql_struct!(
	Table("Files")
	ID("ID")
	pub struct File<Sqlite>{
		i32,
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

impl Insertable<Fields> for File {
	fn sql_types_string(&self, field: Fields) -> String {
		match field{
			Fields::id => self.id.to_string(),
			Fields::Filename => format!("'{}'",self.Filename),
			Fields::Content => format!("'{}'",self.Content),
			Fields::MimeType => {
				match self.MimeType.as_ref() {
					None => "NULL".to_string(),
					Some(mime) => format!("'{}'", mime)
				}
			}
		}
	}
}


impl File {
	pub async fn insert_for_owner(self, db: &mut PoolConnection<Sqlite>, user: &User)->Result<UserFiles,sqlx::Error>{
		match self.insert(db).await{
			Ok(file) => {
				let UserFile = UserFiles::new(user.id, file.id, true);
				match UserFile.insert(db).await{
					Ok(uf) => Ok(uf),
					Err(err0) => Err(err0)
				}
			},
			Err(err) => Err(err)
		}
	}

	pub async fn get_for_user(db: &mut PoolConnection<Sqlite>, user: &User) -> [Vec<File>; 2] {
		UserFiles::get_for_user(db, &user).await
	}

	pub async fn delete_file_from_user(self,db: &mut PoolConnection<Sqlite>, jar:&CookieJar<'_>)->Result<(),String>{
		todo!()
		/*match User::get_from_cookies(db,jar).await{
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
		}*/
	}

	pub async fn change_filename(&mut self,db: &mut PoolConnection<Sqlite>, jar:&CookieJar<'_>, new_filename:String)->Result<(), &'static str>{
		todo!()
		/*match User::get_from_cookies(db, jar).await{
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
		}*/
	}
}