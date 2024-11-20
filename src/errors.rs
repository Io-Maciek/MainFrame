use std::fmt;

use crate::user_maker::UserMaker;
use crate::users::User;

#[derive(Debug)]
pub enum RegisterError {
    TooShortUsername(usize),
    TooLongUsername(usize),
    UsernameContainsWhiteSpace,
    TooShortPassword(usize),
    TooLongPassword(usize),
    PasswordContainsWhiteSpace,
}

impl std::error::Error for RegisterError {}

impl fmt::Display for RegisterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegisterError::TooShortUsername(_) => write!(f, "Nick musi mieć przynajmniej 4 znaki."),
            RegisterError::TooLongUsername(_) => write!(f, "Nick musi być krótszy niż 20 znaków."),
            RegisterError::UsernameContainsWhiteSpace => write!(f, "Nick nie może zawierać spacji"),
            RegisterError::TooShortPassword(_) => {
                write!(f, "Hasło musi mieć przynajmniej 6 znaków.")
            }
            RegisterError::TooLongPassword(_) => write!(f, "Hasło musi być krótsze niż 25 znaków."),
            RegisterError::PasswordContainsWhiteSpace => {
                write!(f, "Hało nie może zawierać spacji.")
            }
        }
    }
}

impl RegisterError {
    /// Creates an error for a username that is too short.
    pub fn too_short_username(length: usize) -> RegisterError {
        RegisterError::TooShortUsername(length)
    }

    /// Creates an error for a username that is too long.
    pub fn too_long_username(length: usize) -> RegisterError {
        RegisterError::TooLongUsername(length)
    }

    /// Creates an error for a username that contains whitespace.
    pub fn username_contains_whitespace() -> RegisterError {
        RegisterError::UsernameContainsWhiteSpace
    }

    /// Creates an error for a password that is too short.
    pub fn too_short_password(length: usize) -> RegisterError {
        RegisterError::TooShortPassword(length)
    }

    /// Creates an error for a password that is too long.
    pub fn too_long_password(length: usize) -> RegisterError {
        RegisterError::TooLongPassword(length)
    }

    /// Creates an error for a password that contains whitespace.
    pub fn password_contains_whitespace() -> RegisterError {
        RegisterError::PasswordContainsWhiteSpace
    }

    pub fn get_reason<'b>(&self) -> &'b str {
        match self {
            RegisterError::TooShortUsername(_) => "short username",
            RegisterError::TooLongUsername(_) => "long username",
            RegisterError::UsernameContainsWhiteSpace => "username with whitespace",
            RegisterError::TooShortPassword(_) => "long password",
            RegisterError::TooLongPassword(_) => "short password",
            RegisterError::PasswordContainsWhiteSpace => "password with whitespace",
        }
    }
}

#[derive(Debug)]
pub enum LoginError<'a> {
    UserDoesNotExist(&'a UserMaker<'a>),
    IncorrectPassword(User),
}

impl std::error::Error for LoginError<'_> {}

// Implement the Display trait to allow pretty printing of the error.
impl fmt::Display for LoginError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoginError::UserDoesNotExist(user_maker) => {
                write!(f, "Użytkownik o nicku {} nie istnieje.", user_maker.uname)
            }
            LoginError::IncorrectPassword(user) => write!(
                f,
                "Podano nieprawidłowe hasło dlsa użytkownika {}.",
                user.Username
            ),
        }
    }
}

impl<'a> LoginError<'a> {
    /// Creates a `UserDoesNotExist` error.
    pub fn user_does_not_exist(user_maker: &'a UserMaker<'_>) -> Self {
        LoginError::UserDoesNotExist(user_maker)
    }

    /// Creates an `IncorrectPassword` error.
    pub fn incorrect_password(user: User) -> Self {
        LoginError::IncorrectPassword(user)
    }

    pub fn get_username(&self) -> String {
        match self {
            LoginError::UserDoesNotExist(user_maker) => user_maker.uname.to_string(),
            LoginError::IncorrectPassword(user) => user.Username.clone(),
        }
    }

    pub fn get_reason<'b>(&self) -> &'b str {
        match self {
            LoginError::UserDoesNotExist(_) => "user does not exist",
            LoginError::IncorrectPassword(_) => "incorrect password",
        }
    }
}

pub fn get_message_from_reason<'a>(reason: &'a str, data: Option<&'a str>) -> Option<String> {
    match reason {
        "user does not exist" => Some(format!(
            "Użytkownik o nicku <strong>{}</strong> nie istnieje.",
            data.unwrap()
        )),
        "incorrect password" => Some(format!(
            "Podano nieprawidłowe hasło dla użytkownika <strong>{}</strong>.",
            data.unwrap()
        )),
        _ => None,
    }
}
