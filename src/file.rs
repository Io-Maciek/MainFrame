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
		match field {
			Fields::id => self.id.to_string(),
			Fields::Filename => format!("'{}'", self.Filename),
			Fields::Content => format!("'{}'", self.Content),
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
	pub async fn insert_for_owner(self, db: &mut PoolConnection<Sqlite>, user: &User) -> Result<UserFiles, sqlx::Error> {
		match self.insert(db).await {
			Ok(file) => {
				let UserFile = UserFiles::new(user.id, file.id, true);
				match UserFile.insert(db).await {
					Ok(uf) => Ok(uf),
					Err(err0) => Err(err0)
				}
			}
			Err(err) => Err(err)
		}
	}

	/// * first '\[0\]' element in array is File vector that provided use has ownership of.
	/// * second '\[1\]' element in array is File vector with files shared to this user.
	pub async fn get_for_user(db: &mut PoolConnection<Sqlite>, user: &User) -> [Vec<File>; 2] {
		UserFiles::get_for_user(db, &user).await
	}

	pub async fn delete_file_from_user(self, db: &mut PoolConnection<Sqlite>, jar: &CookieJar<'_>) -> Result<String, String> {
		match User::get_from_cookies(db, jar).await {
			None => Err("Należy się zalogować".to_string()),
			Some(user) => {
				println!("ZALOGOWANY");
				match UserFiles::delete(db, &user, &self).await {
					Ok(owner_value) => {
						if owner_value{
							Ok(String::from(format!("Plik '{}' został pomyślnie usunięty",self.Filename)))
						}else{
							Ok(String::from(format!("Wyłączono się z udostępniania pliku '{}'",self.Filename)))
						}
					},
					Err(err) => Err(String::from("Nie masz dostępu do tego pliku!"))
				}
			}
		}
	}

	pub async fn change_filename(&mut self, db: &mut PoolConnection<Sqlite>, jar: &CookieJar<'_>, new_filename: String) -> Result<(), &'static str> {
		match User::get_from_cookies(db, jar).await {
			None => Err("Należy się zalogować"),
			Some(user) => {
				match UserFiles::get_from_user_and_file(&mut *db, &user, &self).await {
					Ok(uf) => {
						if uf.Owner == true {
							self.Filename = new_filename;
							&self.update(db).await;
							Ok(())
						} else {
							Err("Nie jesteś właścicielem tego pliku!")
						}
					}
					Err(er) =>{
						println!("{}",format!("{:?}", er));
						Err("Nie masz dostępu do tego pliku!")
					}
				}
			}
		}
	}
}