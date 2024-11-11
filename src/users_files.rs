use rocket::form::validate::Contains;
use crate::{File, sql_struct, User};
use crate::sql_traits::{Insertable, Queryable};
use crate::SQL;
use rocket_db_pools::Connection;
use sqlx::{Error,  Sqlite}; //Mssql
use sqlx::pool::PoolConnection;
use rocket::serde::Serialize;


sql_struct!(
	Table("UserFiles")
	ID("ID")
	pub struct UserFiles<Sqlite>{
		i32,
		pub user_id:i32,
		pub file_id:i32,
		pub owner:bool
	}
);

impl Insertable<Fields> for UserFiles {
	fn sql_types_string(&self, field: Fields) -> String {
		match field {
			Fields::id => self.id.to_string(),
			Fields::user_id => self.user_id.to_string(),
			Fields::file_id => self.file_id.to_string(),
			Fields::owner => match self.owner {
				true => String::from("1"),
				false => String::from("0"),
			}
		}
	}
}

impl UserFiles {
	pub async fn get_from_user_and_file(db: &mut PoolConnection<Sqlite>, user: &User, file: &File) -> Result<UserFiles, sqlx::Error>{
		let q = format!(r"SELECT UF.* FROM UserFiles AS UF Join Users AS U ON UF.UserID = U.ID
								JOIN Files AS F ON F.ID = UF.FileID WHERE U.ID = {} AND F.ID = {}",user.id, file.id);
		sqlx::query_as::<_, UserFiles>(&q).fetch_one(db.as_mut()).await
	}

	pub async fn get_for_user(db: &mut PoolConnection<Sqlite>, user: &User) -> [Vec<File>; 2] {
		let q_owner = format!(r"SELECT F.* FROM UserFiles AS UF JOIN Users AS U ON UF.UserID = U.ID JOIN Files AS F ON F.ID = UF.FileID WHERE U.ID = {}
			AND UF.Owner = 1", user.id);
		let q_shared = format!(r"SELECT F.* FROM UserFiles AS UF JOIN Users AS U ON UF.UserID = U.ID JOIN Files AS F ON F.ID = UF.FileID WHERE U.ID = {}
			AND UF.Owner = 0", user.id);

		let f0 = sqlx::query_as::<_, File>(&q_owner).fetch_all(db.as_mut()).await.unwrap();
		let f1 = sqlx::query_as::<_, File>(&q_shared).fetch_all(db.as_mut()).await.unwrap();
		[f0, f1]
	}

	pub async fn get_file(db: &mut PoolConnection<Sqlite>, user: &User, file_id: i32) -> Option<File> {
		let q = format!("SELECT F.* FROM UserFiles AS UF JOIN Users AS U ON UF.UserID = U.ID JOIN Files AS F ON F.ID = UF.FileID WHERE U.ID = {} AND F.ID = {}",
						user.id, file_id);
		sqlx::query_as::<_, File>(&q)
			.fetch_one(db.as_mut()).await.ok()
	}

	pub async fn delete(db: &mut PoolConnection<Sqlite>, user: &User, file: &File) -> Result<bool, String> {
		let q = format!("SELECT UF.* FROM UserFiles AS UF JOIN Users AS U ON UF.UserID = U.ID JOIN Files AS F ON F.ID = UF.FileID WHERE U.ID = {} AND F.ID = {}",
						user.id, file.id);
		match sqlx::query_as::<_, UserFiles>(&q).fetch_one(db.as_mut()).await {
			Err(err) => {
				println!("{:?}", err);
				Err("Nie masz dostępu do tego pliku".to_string())
			}
			Ok(uf) => {
				match uf.owner == true {
					true => {
						let q_del_mine = format!("DELETE FROM UserFiles WHERE FileID = {}", file.id);
						let deb = sqlx::query_as::<_, UserFiles>(&q_del_mine).fetch_optional(db.as_mut()).await;
						println!("\t{:?}", &deb);

						let deb_file = sqlx::query_as::<_,File>(&format!("DELETE FROM Files WHERE ID={}",file.id)).fetch_all(db.as_mut()).await;
						println!("\t{:?}", &deb_file);

						Ok(true)
					}
					false => {
						println!("Usuwam udostępnienie!");
						let q_del_share = format!("DELETE FROM UserFiles WHERE ID = {}", uf.id);
						let deb = sqlx::query_as::<_, UserFiles>(&q_del_share).fetch_optional(db.as_mut()).await;
						println!("\t{:?}", &deb);
						Ok(false)
					}
				}
			}
		}
	}

	pub async fn sharing_users_of_file(&self,db: &mut PoolConnection<Sqlite>)->Result<Vec<User>,String>{
		if self.owner == true{
			let q = format!("SELECT * FROM UserFiles as UF JOIN Users AS U ON U.ID = UF.UserID WHERE UF.FileID = {} AND Owner = 0",self.file_id);
			match sqlx::query_as::<_, User>(&q).fetch_all(db.as_mut()).await{
				Ok(users) => Ok(users),
				Err(er) => Err(format!("{:?}",er))
			}
		}else{
			Err("Nie jesteś właścicielem pliku!".to_string())
		}
	}

	pub async fn add_shared_user(db: &mut PoolConnection<Sqlite>, user_owner: &User, file: &File, new_user: String)->Result<(), String>{
		if user_owner.username.ne(&new_user) {
			match UserFiles::get_from_user_and_file(db, &user_owner, &file).await {
				Ok(uf) => {
					if uf.owner == true { // adding as owner is OK

						let q0 = format!("SELECT * FROM Users WHERE Username = '{}'", new_user);
						match sqlx::query_as::<_, User>(&q0).fetch_one(db.as_mut()).await {
							Ok(shared_user) => { // user with provided username exist

								// check if provided user is not already added to this file
								if !uf.sharing_users_of_file(db).await.unwrap().contains(&shared_user) {
									let new_sharing = UserFiles::new(shared_user.id, file.id, false);
									let _ = new_sharing.insert(db).await;
									Ok(())
								} else {
									Err(format!("Plik <strong>{}</strong> był już udostępniony użytkownikowi <strong>{}</strong>", file.filename, new_user))
								}
							}
							Err(er0) => { // user with provided nick does not exist
								Err(format!("Użytkownik o nicku <strong>{}</strong> nie istnieje!", new_user))
							}
						}
					} else { // adding as someone else is just strange
						Err(format!("Nie jesteś właścicielem pliku <strong>{}</strong>!", file.filename))
					}
				}
				Err(er) => {
					Err(format!("Nie masz dostępu do pliku <strong>{}</strong>", file.filename))
				}
			}
		}else{
			Err(String::from("Nie możesz udostępnić pliku samemu sobie!"))
		}
	}
}