use std::fmt::{Display, Formatter, write};
use crate::{create, File};
use crate::sql_traits::{Insertable, Queryable};
use crate::SQL;
use rocket_db_pools::Connection;
use data_encoding::HEXUPPER;
use std::num::NonZeroU32;
use std::slice::Join;
use ring::{digest, pbkdf2, rand};
use ring::error::Unspecified;
use ring::rand::SecureRandom;
use rocket::http::{Cookie, CookieJar};
use crate::user_maker::UserMaker;
use sqlx::pool::PoolConnection;
use sqlx::{Error, Sqlite};

create!(
	#[Table("Users")]
	pub struct User<Sqlite>{
		i32,
		pub Username:String,
		pub Hash:String,
		pub Salt:String,
		SessionID:Option<String>
	}
);

impl Insertable for User {
	fn get_insert_string(&self) -> String {
		format!(r"INSERT INTO Users (Username, Hash, Salt) VALUES ('{}','{}','{}') RETURNING *",
				&self.Username, &self.Hash, &self.Salt)
	}

	fn get_update_string(&self) -> String {
		let sess_id = match &self.SessionID {
			None => "null".to_string(),
			Some(sess) => format!("'{}'", sess)
		};
		format!(r"UPDATE Users SET Username='{}', Hash='{}', Salt='{}', SessionID={} WHERE ID = {} RETURNING *",
				&self.Username, &self.Hash, &self.Salt, sess_id, &self.ID)
	}
}

impl Display for User {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", &self.Username)
	}
}

impl User {
	pub async fn get_files(&self, db: &mut PoolConnection<Sqlite>) -> Vec<File> {
		File::get_for_user(db, &self).await
	}

	pub async fn get_file(&self, file_id : i32,db: &mut PoolConnection<Sqlite>)->Option<File>{
		sqlx::query_as::<_, File>(&format!("SELECT F.* FROM Files AS F JOIN Users AS U ON F.UserID=U.ID WHERE U.ID={} AND F.ID={}",&self.ID,file_id))
			.fetch_one(db).await.ok()
	}

	pub async fn create_new_session(&mut self, db: &mut PoolConnection<Sqlite>, jar: &CookieJar<'_>) {
		let rng = ring::rand::SystemRandom::new();
		let mut s = [0u8; 127];
		rng.fill(&mut s).unwrap();
		let encoded_session = HEXUPPER.encode(&s);

		self.SessionID = Some(encoded_session.clone());
		self.update(db).await;

		jar.add(Cookie::build("session_id", encoded_session).http_only(true).finish());
	}

	pub fn new(username: String, hash: String, salt: String) -> User {
		User {
			ID: -1,
			Username: username,
			Salt: salt,
			Hash: hash,
			SessionID: None,
		}
	}

	pub async fn get_from_cookies(db: &mut PoolConnection<Sqlite>, jar: &CookieJar<'_>) -> Option<User> {
		match jar.get("session_id") {
			None => None,
			Some(session_id) => {
				match sqlx::query_as::<_, User>(&format!("SELECT * FROM Users WHERE SessionID='{}'", session_id.value())).fetch_one(&mut *db).await.ok() {
					Some(user) => Some(user),
					None => None
				}
			}
		}
	}

	pub async fn logout(mut db: Connection<SQL>, jar: &CookieJar<'_>) {
		match jar.get("session_id") {
			None => {}
			Some(sess_id_jar) => {
				if let Some(mut user) = User::get_from_cookies(&mut *db, jar).await {
					user.SessionID = None;
					user.update(&mut *db).await;
				}
				jar.remove(sess_id_jar.clone());
			}
		}
	}
}
