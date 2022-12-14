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
struct Message<'a> {
	color: &'a str,
	text: String,
}

impl<'a> Message<'a> {
	pub fn get_from_flash(flash: Option<FlashMessage<'_>>) -> Option<Message> {
		if let Some(_f) = flash {
			let fl = _f.into_inner();
			if &fl.0 == "error" {
				Some(Message { color: "danger", text: fl.1 })
			} else if &fl.0 == "success" {
				Some(Message { color: "success", text: fl.1 })
			} else {
				None
			}
		} else {
			None
		}
	}
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


			Template::render("index_niezalogowany", context! {
					title: "MainFrame",
					message : Message::get_from_flash(flash),
				})
		}
		Some(user) => {
			/*
			//
				GŁÓWNA STRONA DLA ZALOGOWANYCH
			//
			 */

			let files = user.get_files(&mut *db).await;

			let mut sharing_info = Vec::new();

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
				files_shared: &files[1],
				message: Message::get_from_flash(flash),
			})
		}
	}
}

#[get("/delete_sharing/<file_id>/<username>")]
async fn delete_sharing(jar: &CookieJar<'_>, mut db: Connection<SQL>, username: String, file_id: i32) -> Flash<Redirect> {
	match File::get_one(&mut *db, file_id).await {
		Ok(file) => {
			match User::get_from_cookies(&mut *db, jar).await {
				None => Flash::error(Redirect::to(uri!(index)), "Należy się zalogować!"),
				Some(owner) => {
					match UserFiles::get_from_user_and_file(&mut *db, &owner, &file).await {
						Ok(uf_owner) => {
							if uf_owner.Owner == true {
								let q = format!(r"DELETE FROM UserFiles WHERE ID = (SELECT UF.ID FROM UserFiles AS UF JOIN Files AS F ON F.ID = UF.FileID
														JOIN Users AS U ON U.ID = UF.UserID WHERE F.ID = {} AND U.Username = '{}'
														AND Owner = 0)", file_id, username);
								match sqlx::query_as::<_, UserFiles>(&q).fetch_optional(&mut *db).await {
									Ok(k) => {
										Flash::success(Redirect::to(uri!(index)),
													   format!("Przestano udostępniać plik <strong>{}</strong> użytkownikowi <strong>{}</strong>", file.Filename, username))
									}
									Err(er) => Flash::error(Redirect::to(uri!(index)), format!("ERR: {:?}", er))
								}
							} else {
								Flash::error(Redirect::to(uri!(index)), format!("Nie jesteś właścicielem pliku <strong>{}</strong>!", file.Filename))
							}
						}
						Err(err) => Flash::error(Redirect::to(uri!(index)), format!("ERR: {:?}", err))
					}
				}
			}
		}
		Err(err) => Flash::error(Redirect::to(uri!(index)), "Plik nie istnieje!")
	}
}

#[post("/add_new_sharing_user?<file_id>", data = "<username>")]
async fn add_new_sharing_user(jar: &CookieJar<'_>, mut db: Connection<SQL>, username: Form<String>, file_id: i32) -> Flash<Redirect> {
	match User::get_from_cookies(&mut *db, jar).await {
		None => Flash::error(Redirect::to(uri!(index)), "Należy się zalogować!"),
		Some(user_owner) => {
			let f = File::get_one(&mut *db, file_id).await.unwrap();
			let filename = &f.Filename;
			match UserFiles::add_shared_user(&mut *db, &user_owner, &f, username.clone()).await {
				Ok(_) => Flash::success(Redirect::to(uri!(index)), format!("Udostępniono plik <strong>{}</strong> użytkownikowi <strong>{}</strong>", filename, username.into_inner())),
				Err(err) => Flash::error(Redirect::to(uri!(index)), err),
			}
		}
	}
}

#[post("/plik", data = "<data>")]
async fn send_file(jar: &CookieJar<'_>, mut db: Connection<SQL>, content_type: &ContentType, data: Data<'_>) -> Flash<Redirect> {
	//TODO przedstawić graficznie lepiej wysyłanie plików
	match User::get_from_cookies(&mut *db, jar).await {
		None => Flash::error(Redirect::to(uri!(index)), "Należy się zalogować!"),
		Some(user) => {
			let options = MultipartFormDataOptions::with_multipart_form_data_fields(
				vec![
					MultipartFormDataField::file("myfile").size_limit(10_000_000).content_type_by_string(Some(mime::STAR_STAR)).unwrap(),//10MB
				]
			);

			match MultipartFormData::parse(content_type, data, options).await {
				Ok(multipart_form_data) => {
					match multipart_form_data.files.get("myfile") {
						None => Flash::error(Redirect::to(uri!(index)), "Należy przesłać plik."),
						Some(files) => {
							let file = &files[0];
							let pdf = HEXUPPER.encode(&std::fs::read(&file.path).unwrap());

							match File::new(file.file_name.as_ref().unwrap().clone(),
											pdf, file.content_type.as_ref().map(|x| x.to_string()))
								.insert_for_owner(&mut *db, &user).await
							{
								Ok(_) => Flash::success(Redirect::to(uri!(index)), format!("Plik <strong>{}</strong> został przesłany!", file.file_name.as_ref().unwrap())),
								Err(e) => Flash::error(Redirect::to(uri!(index)), format!("{:?}", e)),
							}
						}
					}
				}
				Err(err) => Flash::error(Redirect::to(uri!(index)), "Za duży plik! <i>(10MB)</i>")
			}
		}
	}
}

#[get("/delete/<file_id>")]
async fn delete_file(jar: &CookieJar<'_>, mut db: Connection<SQL>, file_id: i32) -> Flash<Redirect> {
	match File::get_one(&mut *db, file_id).await {
		Err(_) => Flash::error(Redirect::to(uri!(index)), "Plik nie istnieje!"),
		Ok(file) => {
			match file.delete_file_from_user(&mut *db, jar).await {
				Ok(k) => Flash::success(Redirect::to(uri!(index)), k),
				Err(mess) => Flash::error(Redirect::to(uri!(index)), mess)
			}
		}
	}
}

#[get("/change_filename/<new_filename>/<file_id>")]
async fn change_filename(jar: &CookieJar<'_>, mut db: Connection<SQL>, new_filename: String, file_id: i32) -> Result<Redirect, Flash<Redirect>> {
	match File::get_one(&mut *db, file_id).await {
		Err(_) => Err(Flash::error(Redirect::to(uri!(index)), "Plik nie istnieje!")),
		Ok(mut file) => {
			match file.change_filename(&mut *db, jar, new_filename).await {
				Ok(_) => Ok(Redirect::to(uri!(index))),
				Err(mess) => Err(Flash::error(Redirect::to(uri!(index)), mess))
			}
		}
	}
}

#[get("/get/<file_id>/<file_name>")]
async fn get_file_by_id(jar: &CookieJar<'_>, mut db: Connection<SQL>, file_id: i32, file_name: String) -> Result<RawHtml<String>, Flash<Redirect>> { //Result<Vec<u8>, &'a str> {
	//TODO przedstawić to trochę lepiej

	//sqlx::query_as::<_, User>("").bind(Vec::<u8>::new());
	match User::get_from_cookies(&mut *db, jar).await {
		None => Err(Flash::error(Redirect::to(uri!(index)), "Należy się zalogować!")),//niezalogowany
		Some(user) => {                //zalogowany
			match user.get_file(file_id, &mut *db).await {
				None => Err(Flash::error(Redirect::to(uri!(index)), "Nie masz dostępu do tego pliku!")),    //nie znaleziono pliku
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
	let u = maker_user.uname;
	match maker_user.into_inner().create_user() {
		Ok(user) => {
			match user.insert(&mut *db).await {
				Ok(u) => Flash::success(Redirect::to(uri!(index)), format!(" Pomyślnie utworzono użytkownika <strong>{}</strong>", u.Username)),
				Err(_) => Flash::error(Redirect::to(uri!(index)), format!("Nick <strong>{}</strong> jest już zajęty!", u))
			}
		}
		Err(err) => Flash::error(Redirect::to(uri!(index)), err)
	}
}

#[post("/login", data = "<maker_user>")]
async fn index_login(mut db: Connection<SQL>, jar: &CookieJar<'_>, maker_user: Form<UserMaker<'_>>) -> Result<Redirect, Flash<Redirect>> {
	match maker_user.into_inner().check_user_login(&mut *db).await {
		Ok(mut user) => {
			user.create_new_session(&mut *db, jar).await;
			Ok(Redirect::to(uri!(index)))
		}
		Err(err) => Err(Flash::error(Redirect::to(uri!(index)), err))
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
		.mount("/", routes![delete_sharing, add_new_sharing_user, favicon, index, index_post,index_login,index_logout,get_file_by_id,send_file,delete_file,change_filename])
		.attach(Template::custom(|eng| {
			eng.handlebars.register_helper("mod", Box::new(hbs_helpers::modulo));
		}))
}


static_response_handler! {
    "/favicon.png" => favicon => "favicon",
}