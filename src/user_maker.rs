use crate::users::User;
use data_encoding::HEXUPPER;
use std::num::NonZeroU32;
use ring::rand::SecureRandom;
use ring::{digest, pbkdf2, rand};
use ring::error::Unspecified;
use rocket::form::Form;
use rocket_db_pools::Connection;
use sqlx::{Sqlite, Mssql};
use crate::{PoolConnection, SQL};

#[derive(FromForm, Clone, Debug)]
pub struct UserMaker<'a>{
	uname: &'a str,
	pwd: &'a str
}

impl UserMaker<'_>{
	pub fn create_user(self)->Result<User, &'static str>{
		if &self.uname.len()<=&3{
			return Err("Nick musi mieć przynajmniej 4 znaki")
		}else if &self.uname.len()>=&20{
			return Err("Nick musi być krótszy niż 20 znaków")
		}else if self.uname.contains(char::is_whitespace){
			return Err("Nick nie może zawierać spacji")
		}else if &self.pwd.len()<=&5{
			return Err("Hasło musi mieć przynajmniej 6 znaków")
		}else if &self.pwd.len()>=&25{
			return Err("Hasło musi być krótsze niż 25 znaków")
		}else if self.pwd.contains(char::is_whitespace){
			return  Err("Hasło nie może zawierać spacji")
		}


		const CREDENTIAL_LEN: usize = digest::SHA512_OUTPUT_LEN;
		let n_iter = NonZeroU32::new(100_000).unwrap();
		let rng = ring::rand::SystemRandom::new();
		let mut s = [0u8; CREDENTIAL_LEN];
		rng.fill(&mut s).unwrap();

		let mut pbkdf2_hash = [0u8; CREDENTIAL_LEN];
		pbkdf2::derive(
			pbkdf2::PBKDF2_HMAC_SHA512,
			n_iter,
			&s,
			&self.pwd.as_bytes(),
			&mut pbkdf2_hash,
		);

		Ok(User::new(self.uname.to_owned(),HEXUPPER.encode(&pbkdf2_hash),HEXUPPER.encode(&s),None))
	}

	pub async fn check_user_login(self,db : &mut PoolConnection<Sqlite>)->Result<User, String>{
		let user_check = sqlx::query_as::<_, User>(&format!("SELECT * FROM Users WHERE Username='{}'",&self.uname))
			.fetch_one(db).await.ok();
		match user_check{
			None => Err(format!("Użytkownik o nicku \"{}\" nie istnieje",&self.uname)),
			Some(u) => {
				if self==u{
					Ok(u)
				}else{
					Err(String::from("Podano nieprawidłowe hasło"))
				}
			}
		}
	}
}

impl PartialEq<User> for UserMaker<'_> {
	fn eq(&self, other: &User) -> bool {
		const CREDENTIAL_LEN: usize = digest::SHA512_OUTPUT_LEN;
		let n_iter = NonZeroU32::new(100_000).unwrap();

		let mut pbkdf2_hash = [0u8; CREDENTIAL_LEN];
		pbkdf2::derive(
			pbkdf2::PBKDF2_HMAC_SHA512,
			n_iter,
			HEXUPPER.decode(&other.Salt.as_bytes()).unwrap().as_slice(),
			&self.pwd.as_bytes(),
			&mut pbkdf2_hash,
		);


		match &other.Hash.eq(&HEXUPPER.encode(&pbkdf2_hash)) {
			true => true,
			false => false,
		}
	}
}