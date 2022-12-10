use rocket::form::validate::Contains;
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

impl Insertable<Fields> for UserFiles {
	fn sql_types_string(&self, field: Fields) -> String {
		match field {
			Fields::id => self.id.to_string(),
			Fields::UserID => self.UserID.to_string(),
			Fields::FileID => self.FileID.to_string(),
			Fields::Owner => match self.Owner {
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
		sqlx::query_as::<_, UserFiles>(&q).fetch_one(&mut *db).await
	}

	pub async fn get_for_user(db: &mut PoolConnection<Sqlite>, user: &User) -> [Vec<File>; 2] {
		let q_owner = format!(r"SELECT F.* FROM UserFiles AS UF JOIN Users AS U ON UF.UserID = U.ID JOIN Files AS F ON F.ID = UF.FileID WHERE U.ID = {}
			AND UF.Owner = 1", user.id);
		let q_shared = format!(r"SELECT F.* FROM UserFiles AS UF JOIN Users AS U ON UF.UserID = U.ID JOIN Files AS F ON F.ID = UF.FileID WHERE U.ID = {}
			AND UF.Owner = 0", user.id);

		let f0 = sqlx::query_as::<_, File>(&q_owner).fetch_all(&mut *db).await.unwrap();
		let f1 = sqlx::query_as::<_, File>(&q_shared).fetch_all(&mut *db).await.unwrap();
		[f0, f1]
	}

	pub async fn get_file(db: &mut PoolConnection<Sqlite>, user: &User, file_id: i32) -> Option<File> {
		let q = format!("SELECT F.* FROM UserFiles AS UF JOIN Users AS U ON UF.UserID = U.ID JOIN Files AS F ON F.ID = UF.FileID WHERE U.ID = {} AND F.ID = {}",
						user.id, file_id);
		sqlx::query_as::<_, File>(&q)
			.fetch_one(db).await.ok()
	}

	pub async fn delete(db: &mut PoolConnection<Sqlite>, user: &User, file: &File) -> Result<(), String> {
		let q = format!("SELECT UF.* FROM UserFiles AS UF JOIN Users AS U ON UF.UserID = U.ID JOIN Files AS F ON F.ID = UF.FileID WHERE U.ID = {} AND F.ID = {}",
						user.id, file.id);
		match sqlx::query_as::<_, UserFiles>(&q).fetch_one(&mut *db).await {
			Err(err) => {
				println!("{:?}", err);
				Err("Nie masz dostępu do tego pliku".to_string())
			}
			Ok(UF) => {
				match UF.Owner == true {
					true => {
						let q_del_mine = format!("DELETE FROM UserFiles WHERE FileID = {}", file.id);
						let deb = sqlx::query_as::<_, UserFiles>(&q_del_mine).fetch_optional(&mut *db).await;
						println!("\t{:?}", &deb);

						let deb_file = sqlx::query_as::<_,File>(&format!("DELETE FROM Files WHERE ID={}",file.id)).fetch_all(&mut *db).await;
						println!("\t{:?}", &deb_file);

						Ok(())
					}
					false => {
						println!("Usuwam udostępnienie!");
						let q_del_share = format!("DELETE FROM UserFiles WHERE ID = {}", UF.id);
						let deb = sqlx::query_as::<_, UserFiles>(&q_del_share).fetch_optional(&mut *db).await;
						println!("\t{:?}", &deb);
						Ok(())
					}
				}
			}
		}
	}

	pub async fn sharing_users_of_file(&self,db: &mut PoolConnection<Sqlite>)->Result<Vec<User>,String>{
		if self.Owner == true{
			let q = format!("SELECT * FROM UserFiles as UF JOIN Users AS U ON U.ID = UF.UserID WHERE UF.FileID = {} AND Owner = 0",self.FileID);
			match sqlx::query_as::<_, User>(&q).fetch_all(&mut *db).await{
				Ok(users) => Ok(users),
				Err(er) => Err(format!("{:?}",er))
			}
		}else{
			Err("Nie jesteś właścicielem pliku!".to_string())
		}
	}

	pub async fn add_shared_user(db: &mut PoolConnection<Sqlite>, user_owner: &User, file: &File, new_user: String)->Result<(), String>{
		if user_owner.clone().Username.ne(&new_user) {
			match UserFiles::get_from_user_and_file(db, &user_owner, &file).await {
				Ok(uf) => {
					if uf.Owner == true { // adding as owner is OK

						let q0 = format!("SELECT * FROM Users WHERE Username = '{}'", new_user);
						match sqlx::query_as::<_, User>(&q0).fetch_one(&mut *db).await {
							Ok(shared_user) => { // user with provided username exist

								// check if provided user is not already added to this file
								if !uf.sharing_users_of_file(db).await.unwrap().contains(&shared_user) {
									let new_sharing = UserFiles::new(shared_user.id, file.id, false);
									new_sharing.insert(db).await;
									Ok(())
								} else {
									Err(format!("Plik '{}' był już udostępniony użytkownikowi '{}'", file.Filename, new_user))
								}
							}
							Err(er0) => { // user with provided nick does not exist
								Err(format!("Użytkownik o nicku '{}' nie istnieje!", new_user))
							}
						}
					} else { // adding as someone else is just strange
						Err(format!("Nie jesteś właścicielem pliku '{}'!", file.Filename))
					}
				}
				Err(er) => {
					Err(format!("Nie masz dostępu do pliku '{}'", file.Filename))
				}
			}
		}else{
			Err(String::from("Nie możesz udostępnić pliku samemu sobie!"))
		}
	}
}