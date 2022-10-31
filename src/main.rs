#![feature(slice_concat_trait)]

use std::future::Future;
use data_encoding::HEXUPPER;
use rocket::{Build, Rocket};
use rocket::form::Form;
use rocket::http::CookieJar;
use rocket::response::content::RawHtml;
use rocket::response::Redirect;
use crate::sql_connectivity::SQL;
use rocket_db_pools::{Connection, Database};
use sqlx::pool::PoolConnection;
use sqlx::Sqlite;
use crate::file::File;
use crate::sql_traits::Queryable;
use crate::user_maker::UserMaker;
use crate::users::User;

#[macro_use]
extern crate rocket;

mod html_macros;
mod sql_connectivity;
mod sql_traits;
mod users;
mod user_maker;
mod file;

#[get("/")]
async fn index(jar: &CookieJar<'_>, mut db: Connection<SQL>) -> RawHtml<String> {
	page_template(
		match User::get_from_cookies(&mut *db, &jar.clone()).await {
			None => {
				format!("{}<br><br>{}",
					tag!(form action="/" method="POST",
						tag!(label, "Username:"),
						tag_str!("input type='text' id='uname' name='uname'"),
						tag!(label, "Password:"),
						tag_str!("input type='text' id='pwd' name='pwd'"),
						tag_str!("input class='btn btn-success' type='submit' value='Stwórz'")
					),
						tag!(form action="/login" method="POST",
							tag!(label, "Username:"),
							tag_str!("input type='text' id='uname' name='uname'"),
							tag!(label, "Password:"),
							tag_str!("input type='text' id='pwd' name='pwd'"),
							tag_str!("input class='btn btn-success' type='submit' value='Zaloguj się'")
						)
				)
			},
			Some(user) => {
				let mut files_str = String::new();
				for mut file in &user.get_files(&mut *db).await {

					files_str+=
						&tag_str!(format!("a href='/get/{}'",&file.ID),
							tag!(h5, &file.to_string())
						);
				}
				format!("{}<br>{}",
					tag!(h5,format!("Zalogowano! {}",user)),
					files_str
				)

			}
	},jar, db).await
}

#[get("/get/<file_id>")]
async fn get_file_by_id<'a>(jar: &'a CookieJar<'_>, mut db: Connection<SQL>, file_id: i32)->Result<Vec<u8>,&'a str>{
	//TODO przedstawić to troche lepiej i wgl pozwolić na dodawanie nowych plików
	if let Some(user) = User::get_from_cookies(&mut *db, jar).await{
		return match user.get_file(file_id, &mut *db).await {
			None => Err("nie masz dostępu do tego pliku!"),
			Some(file) => Ok(HEXUPPER.decode(&file.Content.as_bytes()).unwrap())
		}
	}
	Err("Musisz się zalogować")
}

#[post("/", data = "<maker_user>")]
async fn index_post(jar: &CookieJar<'_>, mut db: Connection<SQL>, maker_user: Form<UserMaker<'_>>) -> RawHtml<String> {
	let user = maker_user.into_inner().create_user();
	let body = format!("{}", user.insert(&mut *db).await);
	page_template(body,jar, db).await
}

#[post("/login", data = "<maker_user>")]
async fn index_login(mut db: Connection<SQL>, jar: &CookieJar<'_>, maker_user: Form<UserMaker<'_>>) -> Redirect {
	match maker_user.into_inner().check_user_login(&mut *db).await {
		Ok(mut user) => { user.create_new_session(&mut *db, jar).await; }
		Err(err) => println!("{}", err)
	}

	Redirect::to(uri!(index))
}

#[get("/logout")]
async fn index_logout(mut db: Connection<SQL>, jar: &CookieJar<'_>) -> Redirect {
	User::logout(db, jar).await;
	Redirect::to(uri!(index))
}

#[launch]
fn rocket() -> Rocket<Build> {
/*	let pdf = std::fs::read("c.pdf").unwrap();
	println!("''{}''",HEXUPPER.encode(&pdf));*/

	let figment = rocket::Config::figment().merge(("address", "0.0.0.0"))
		.merge(("databases.MainFrame", rocket_db_pools::Config {
			url: "MainFrame.db".into(),
			min_connections: None,
			max_connections: 1024,
			connect_timeout: 5,
			idle_timeout: None,
		}));

	rocket::custom(figment).attach(SQL::init()).mount("/", routes![index, index_post,index_login,index_logout,get_file_by_id])
}

async fn page_template<T>(body: T,jar: &CookieJar<'_>, mut db: Connection<SQL>) -> RawHtml<String>
	where T: std::fmt::Display {
	let add = match User::get_from_cookies(&mut *db, jar).await {
		None => "".to_string(),
		Some(user) => format!("{}<br>",
							  tag_str!(format!("a href='/logout'"),
								tag!(h5,user.Username)
							)
		)
	};

	html!(
		tag!(head,
			tag!(meta charset="utf-8"),
			tag!(meta name="viewport" content="width=device-width, initial-scale=1"),
			tag!(link rel="stylesheet" href="https://maxcdn.bootstrapcdn.com/bootstrap/4.0.0-alpha.6/css/bootstrap.min.css"),
            tag!(script src="https://code.jquery.com/jquery-3.1.1.slim.min.js"),
            tag!(script src="https://cdnjs.cloudflare.com/ajax/libs/tether/1.4.0/js/tether.min.js"),
            tag!(script src="https://maxcdn.bootstrapcdn.com/bootstrap/4.0.0-alpha.6/js/bootstrap.min.js"),
            tag!(title, "Main Frame")
		),// : </head>
        tag!(body style="background-color:gray",
        	tag!(div style="padding: 15px;" class="container",
				add,
				body
			)// : </div>
		)// : </body>
	)
}