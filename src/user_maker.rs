use crate::errors::{self, LoginError};
use crate::users::User;
use crate::PoolConnection;
use data_encoding::HEXUPPER;
use ring::rand::SecureRandom;
use ring::{digest, pbkdf2};
use sqlx::Sqlite; //Mssql
use std::num::NonZeroU32;

#[derive(FromForm, Clone, Debug)]
pub struct UserMaker<'a> {
    pub uname: &'a str,
    pwd: &'a str,
}

impl UserMaker<'_> {
    pub fn create_user(self) -> Result<User, errors::RegisterError> {
        if &self.uname.len() <= &3 {
            return Err(errors::RegisterError::too_short_username(self.uname.len()));
        } else if &self.uname.len() >= &20 {
            return Err(errors::RegisterError::too_long_username(self.uname.len()));
        } else if self.uname.contains(char::is_whitespace) {
            return Err(errors::RegisterError::username_contains_whitespace())
        } else if &self.pwd.len() <= &5 {
            return Err(errors::RegisterError::too_short_password(self.pwd.len()));
        } else if &self.pwd.len() >= &25 {
            return Err(errors::RegisterError::too_long_password(self.pwd.len()));
        } else if self.pwd.contains(char::is_whitespace) {
            return Err(errors::RegisterError::password_contains_whitespace())
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

        Ok(User::new(
            self.uname.to_owned(),
            HEXUPPER.encode(&pbkdf2_hash),
            HEXUPPER.encode(&s),
            None,
        ))
    }

    pub async fn check_user_login(
        &self,
        db: &mut PoolConnection<Sqlite>,
    ) -> Result<User, LoginError<'_>> {
        let user_check = sqlx::query_as::<_, User>(&format!(
            "SELECT * FROM Users WHERE Username='{}'",
            &self.uname
        ))
        .fetch_one(db.as_mut())
        .await
		.ok();

        match user_check {
            None => Err(LoginError::user_does_not_exist(&self)),
            Some(u) => {
                if self == &u {
                    Ok(u)
                } else {
                    Err(LoginError::incorrect_password(u))
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
