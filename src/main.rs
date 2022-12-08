use std::future::Future;
use data_encoding::HEXUPPER;
use rocket::{Build, Data, Request, response, Rocket, State};
use rocket::form::Form;
use rocket::fs::NamedFile;
use rocket::http::{CookieJar, Status};
use rocket::http::ext::IntoCollection;
use rocket::response::content::RawHtml;
use rocket::response::{Flash, Redirect, Responder};
use crate::sql_connectivity::SQL;
use rocket_db_pools::{Connection, Database};
use sqlx::pool::PoolConnection;
use sqlx::{Error, pool, Sqlite, SqlitePool};
use crate::file::File;
use crate::sql_traits::Queryable;
use crate::user_maker::UserMaker;
use crate::users::User;
use rocket::serde::Serialize;
use rocket::http::ContentType;
use rocket::request::FlashMessage;
use rocket_multipart_form_data::{FileField, mime, MultipartFormData, MultipartFormDataError, MultipartFormDataField, MultipartFormDataOptions};
use rocket_dyn_templates::{Template, handlebars, context};
use handlebars::{Handlebars, JsonRender};
use rocket_include_static_resources::{static_resources_initializer, static_response_handler};
use crate::handlebars::HelperDef;


//TODO deleting user from sharing file in owner account
//TODO searching shared file by name (works only for owned files)

#[macro_use]
extern crate rocket;


mod html_macros;
mod sql_connectivity;
mod sql_traits;
mod users;
mod user_maker;
mod file;
mod hbs_helpers;
mod users_files;

#[derive(Serialize)]
struct Message {
	html_color: String,
	message: String,
}

#[get("/")]
async fn index(jar: &CookieJar<'_>, mut db: Connection<SQL>, flash: Option<FlashMessage<'_>>) -> Template {
	match User::get_from_cookies(&mut *db, &jar.clone()).await {
		None => {
			/*
			//
				GŁÓWNA STRONA DLA NIEZALOGOWANEGO UŻYTKOWNIKA
			//
			 */
			let mut msg_login = None::<Message>;
			let mut msg_rej = None::<Message>;

			if flash.is_some() {
				let fl = flash.unwrap().into_inner();
				match &fl.0 == "error_log" {
					true => msg_login = Some(Message { html_color: "danger".to_string(), message: fl.1 }),
					false => {
						match &fl.0 == "error_rej" {
							true => msg_rej = Some(Message { html_color: "danger".to_string(), message: fl.1 }),
							false => msg_rej = Some(Message { html_color: "success".to_string(), message: fl.1 }),
						}
					}
				}
			}

			Template::render("index_niezalogowany", context! {
					title: "MainFrame",
					msg_rej : msg_rej,
					msg_login: msg_login
				})
		}
		Some(user) => {
			/*
			//
				GŁÓWNA STRONA DLA ZALOGOWANYCH
			//
			 */

			let files = user.get_files(&mut *db).await;

			let mut sharing_info = Vec::<Vec::<User>>::new();

			for f in &files[0] {
				let d = UserFiles::get_from_user_and_file(&mut *db, &user, f).await.unwrap();
				let mut t = d.sharing_users_of_file(&mut *db).await.unwrap();
				sharing_info.push(t);
			}

			Template::render("index_zalogowany", context! {
				title: "MainFrame",
				user: &user,
				sharing_info: &sharing_info,
				files_owned: &files[0],
				files_shared: &files[1]
			})
		}
	}
}

#[post("/add_new_sharing_user?<file_id>", data="<username>")]
async fn add_new_sharing_user(jar: &CookieJar<'_>, mut db: Connection<SQL>,username: Form<String>, file_id : i32)->Redirect{
	match User::get_from_cookies(&mut *db,jar).await{
		None => println!("NALEŻY SIĘ ZALOGOWAĆ"),
		Some(user_owner) => {
			let f= &File::get_one(&mut *db, file_id).await.unwrap();
			UserFiles::add_shared_user(&mut *db, &user_owner,&f,username.into_inner()).await;

		}
	}

	Redirect::to(uri!(index))
}

#[post("/plik", data = "<data>")]
async fn send_file(jar: &CookieJar<'_>, mut db: Connection<SQL>, content_type: &ContentType, data: Data<'_>) -> String {
	//TODO przedstawić graficznie lepiej wysyłanie plików
	match User::get_from_cookies(&mut *db, jar).await {
		None => {
			String::from("Trzeba być zalogowanym!")
		}
		Some(user) => {
			let options = MultipartFormDataOptions::with_multipart_form_data_fields(
				vec![
					MultipartFormDataField::file("myfile").size_limit(10_000_000).content_type_by_string(Some(mime::STAR_STAR)).unwrap(),//10MB
				]
			);

			match MultipartFormData::parse(content_type, data, options).await {
				Ok(multipart_form_data) => {
					match multipart_form_data.files.get("myfile") {
						None => String::from("Brak pliku"),
						Some(files) => {
							let file = &files[0];
							let pdf = HEXUPPER.encode(&std::fs::read(&file.path).unwrap());

							match File::new(file.file_name.as_ref().unwrap().clone(),
									  pdf,file.content_type.as_ref().map(|x| x.to_string()))
							.insert_for_owner(&mut *db, &user).await
							{
								Ok(_) => format!("Udało się!"),
								Err(e) => {format!("{:?}",e)}
							}

						}
					}
				}
				Err(err) => {
					format!("{:?}", err)
				}
			}
		}
	}
}

#[get("/delete/<file_id>")]
async fn delete_file(jar: & CookieJar<'_>, mut db: Connection<SQL>, file_id: i32)->Result<Redirect, String>{
	match File::get_one(&mut *db, file_id).await {
		Err(_) => Err(String::from("Plik nie istnieje")),
		Ok(file) => {
			match file.delete_file_from_user(&mut *db, jar).await{
				Ok(_)=>Ok(Redirect::to(uri!(index))),
				Err(mess)=>Err(mess)
			}
		}
	}
}

#[get("/change_filename/<new_filename>/<file_id>")]
async fn change_filename(jar: & CookieJar<'_>, mut db: Connection<SQL>,new_filename: String, file_id: i32)->Result<Redirect, &'static str>{
	match File::get_one(&mut *db, file_id).await {
		Err(_) => Err("Plik nie istnieje"),
		Ok(mut file) => {
			match file.change_filename(&mut *db, jar, new_filename).await{
				Ok(_)=>Ok(Redirect::to(uri!(index))),
				Err(mess)=>Err(mess)
			}
		}
	}
}

#[get("/get/<file_id>/<file_name>")]
async fn get_file_by_id<'a>(jar: &'a CookieJar<'_>, mut db: Connection<SQL>, file_id: i32, file_name: String) -> Result<RawHtml<String>, &'a str> { //Result<Vec<u8>, &'a str> {
	//TODO przedstawić to troche lepiej

	//sqlx::query_as::<_, User>("").bind(Vec::<u8>::new());
	match User::get_from_cookies(&mut *db, jar).await {
		None => Err("Musisz się zalogować"),//niezalogowany
		Some(user) => {                //zalogowany
			match user.get_file(file_id, &mut *db).await {
				None => Err("nie masz dostępu do tego pliku!"),    //nie znaleziono pliku
				Some(file) => {                                //plik jest
					let bytes = file.Content.as_bytes();
					let hex = HEXUPPER.decode(bytes).unwrap();
					//Ok(hex)
					Ok(RawHtml(
						format!("
						<iframe frameborder='0' id='ItemPreview' src='' width='98%' height='98%'></iframe>
						<script>
							document.getElementById('ItemPreview').src = 'data:{};base64,{}';
						</script>
						", file.MimeType.unwrap(), base64::encode(&hex))
					))
				}
			}
		}
	}
}

#[post("/", data = "<maker_user>")]
async fn index_post(jar: &CookieJar<'_>, mut db: Connection<SQL>, maker_user: Form<UserMaker<'_>>) -> Flash<Redirect> {
	match maker_user.into_inner().create_user() {
		Ok(user) => {
			let user = user.insert(&mut *db).await.unwrap();
			Flash::success(Redirect::to(uri!(index)), format!(" Pomyślnie utworzono użytkownika \"{}\"", user.Username))
		}
		Err(err) => Flash::new(Redirect::to(uri!(index)), "error_rej", err)
	}
}

#[post("/login", data = "<maker_user>")]
async fn index_login(mut db: Connection<SQL>, jar: &CookieJar<'_>, maker_user: Form<UserMaker<'_>>) -> Result<Redirect, Flash<Redirect>> {
	match maker_user.into_inner().check_user_login(&mut *db).await {
		Ok(mut user) => {
			user.create_new_session(&mut *db, jar).await;
			Ok(Redirect::to(uri!(index)))
		}
		Err(err) => Err(Flash::new(Redirect::to(uri!(index)), "error_log", err))
	}
}

#[get("/logout")]
async fn index_logout(mut db: Connection<SQL>, jar: &CookieJar<'_>) -> Redirect {
	User::logout(db, jar).await;
	Redirect::to(uri!(index))
}

use std::ops::Deref;
use crate::users_files::UserFiles;

#[launch]
fn rocket() -> Rocket<Build> {
	let figment = rocket::Config::figment().merge(("address", "0.0.0.0"))
		.merge(("databases.MainFrame", rocket_db_pools::Config {
			url: "MainFrame.db".into(),
			//url: "mssql://[USER]:[PWD]@localhost:1433/MainFrame".into(),
			min_connections: None,
			max_connections: 1024,
			connect_timeout: 5,
			idle_timeout: None,
		}));
	rocket::custom(figment)
		.attach(SQL::init())
		.attach(static_resources_initializer!(
			"favicon" => "img/favicon.png",
		))
		.mount("/", routes![add_new_sharing_user, favicon, index, index_post,index_login,index_logout,get_file_by_id,send_file,delete_file,change_filename])
		.attach(Template::custom(|eng| {
			eng.handlebars.register_helper("mod", Box::new(hbs_helpers::modulo));
		}))
}

#[allow(dead_code)]
async fn page_template<T>(body: T, jar: &CookieJar<'_>, mut db: Connection<SQL>) -> RawHtml<String>
	where T: std::fmt::Display {
	let add = match User::get_from_cookies(&mut *db, jar).await {
		None => "".to_string(),
		Some(user) =>
			tag!(a href="/logout",
				tag!(h5,user.Username)
			)
				+ "<br>"
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

static_response_handler! {
    "/favicon.png" => favicon => "favicon",
}