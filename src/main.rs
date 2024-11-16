#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use crate::file::File;
use crate::sql_connectivity::SQL;
use crate::sql_traits::Queryable;
use crate::user_maker::UserMaker;
use crate::users::User;
use base64::prelude::*;
use data_encoding::HEXUPPER;
use rocket::form::Form;
use rocket::fs::{relative, FileServer};
use rocket::http::ContentType;
use rocket::http::CookieJar;
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket::serde::Serialize;
use rocket::{Build, Data, Rocket, State};
use rocket_db_pools::{Connection, Database};
use rocket_dyn_templates::{context, handlebars, Template};
use rocket_multipart_form_data::{
    mime, MultipartFormData, MultipartFormDataField, MultipartFormDataOptions,
};
use sqlx::pool::PoolConnection;

#[macro_use]
extern crate rocket;

mod errors;
mod file;
mod hbs_helpers;
mod html_macros;
mod logs;
mod sql_connectivity;
mod sql_traits;
mod user_maker;
mod users;
mod users_files;

#[derive(Serialize)]
struct Message<'a> {
    color: &'a str,
    text: String,
    optional: Option<String>,
}

impl<'a> Message<'a> {
    pub fn get_from_flash(flash: Option<FlashMessage<'_>>) -> Option<Message> {
        if let Some(_f) = flash {
            let fl: (String, String) = _f.into_inner();
            if let Some(text) = errors::get_message_from_reason(&fl.0, Some(&fl.1)) {
                return Some(Message {
                    color: "danger",
                    text: text,
                    optional: None,
                });
            }

            if &fl.0 == "error" {
                Some(Message {
                    color: "danger",
                    text: fl.1,
                    optional: None,
                })
            } else if &fl.0 == "success" {
                Some(Message {
                    color: "success",
                    text: fl.1,
                    optional: None,
                })
            } else if &fl.0 == "error_w_nick" {
                //println!("asdadasdsadsad");
                let splitted = fl.1.split(";;;").collect::<Vec<&str>>();
                Some(Message {
                    color: "danger",
                    text: splitted[0].into(),
                    optional: Some(splitted[1].into()),
                })
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[get("/register")]
async fn register_new(
    jar: &CookieJar<'_>,
    mut db: Connection<SQL>,
    flash: Option<FlashMessage<'_>>,
) -> Result<Template, Redirect> {
    match User::get_from_cookies(&mut *db, &jar.clone()).await {
        None => {
            /*
            //
                GŁÓWNA STRONA DLA NIEZALOGOWANEGO UŻYTKOWNIKA
            //
             */

            Ok(Template::render(
                "rejestracja",
                context! {
                    title: "MainFrame",
                    message: Message::get_from_flash(flash),
                },
            ))
        }
        Some(_) => {
            /*
            //
                GŁÓWNA STRONA DLA ZALOGOWANYCH
                jeżeli zalogowany, to redirect-uj do strony głównej
            //
             */

            Err(Redirect::to(uri!(index)))
        }
    }
}

#[post("/register", data = "<maker_user>")]
async fn register_new_post<'a>(
    mut db: Connection<SQL>,
    maker_user: Form<UserMaker<'_>>,
    logs: &'a State<Log>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let u = maker_user.uname;

    // Attempt to create the user
    let user = maker_user.into_inner()
        .create_user()
        .map_err(|err| {
            logs.register(vec![&u, "register", &err.get_reason(), "0"]);
            Flash::error(Redirect::to(uri!(register_new)), &err.to_string())
        })?;

    // Attempt to insert the user into the database
    user.insert(&mut *db)
        .await
        .map(|u| {
            logs.register(vec![&u.Username, "Utworzono użytkownika."]);
            Flash::success(
                Redirect::to(uri!(index)),
                format!("Pomyślnie utworzono użytkownika <strong>{}</strong>", u.Username),
            )
        })
        .map_err(|_| {
            logs.register(vec![&u, "Nick jest już zajęty."]);
            Flash::error(
                Redirect::to(uri!(register_new)),
                format!("Nick <strong>{}</strong> jest już zajęty!", u),
            )
        })
}


#[get("/")]
async fn index(
    jar: &CookieJar<'_>,
    mut db: Connection<SQL>,
    flash: Option<FlashMessage<'_>>,
) -> Template {
    match User::get_from_cookies(&mut *db, &jar.clone()).await {
        None => {
            /*
            //
                GŁÓWNA STRONA DLA NIEZALOGOWANEGO UŻYTKOWNIKA
            //
             */

            Template::render(
                "index_niezalogowany",
                context! {
                    title: "MainFrame",
                    message : Message::get_from_flash(flash),
                },
            )
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
                let d = UserFiles::get_from_user_and_file(&mut *db, &user, f)
                    .await
                    .unwrap();
                let t = d.sharing_users_of_file(&mut *db).await.unwrap();
                sharing_info.push(t);
            }

            Template::render(
                "index_zalogowany",
                context! {
                    title: "MainFrame",
                    user: &user,
                    sharing_info: &sharing_info,
                    files_owned: &files[0],
                    files_shared: &files[1],
                    message: Message::get_from_flash(flash),
                },
            )
        }
    }
}

#[get("/delete_sharing/<file_id>/<username>")]
async fn delete_sharing(
    jar: &CookieJar<'_>,
    mut db: Connection<SQL>,
    username: String,
    file_id: i32,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    // Fetch the file from the database
    let file = File::get_one(&mut *db, file_id)
        .await
        .map_err(|_| Flash::error(Redirect::to(uri!(index)), "Plik nie istnieje!"))?;

    // Fetch the user from cookies
    let owner = User::get_from_cookies(&mut *db, jar)
        .await
        .ok_or_else(|| Flash::error(Redirect::to(uri!(index)), "Należy się zalogować!"))?;

    // Attempt to fetch the UserFiles entry for the user and file
    let uf_owner = UserFiles::get_from_user_and_file(&mut *db, &owner, &file)
        .await
        .map_err(|err| Flash::error(Redirect::to(uri!(index)), format!("ERR: {:?}", err)))?;

    // Check if the user is the owner
    if uf_owner.Owner {
        // Construct the SQL delete query
        let q = format!(
            r"DELETE FROM UserFiles WHERE ID = (SELECT UF.ID FROM UserFiles AS UF JOIN Files AS F ON F.ID = UF.FileID
                                            JOIN Users AS U ON U.ID = UF.UserID WHERE F.ID = {} AND U.Username = '{}'
                                            AND Owner = 0)",
            file_id, username
        );

        // Execute the delete query
        sqlx::query_as::<_, UserFiles>(&q)
            .fetch_optional(db.as_mut())
            .await
            .map(|_| Ok(Flash::success(
                Redirect::to(uri!(index)),
                format!(
                    "Przestano udostępniać plik <strong>{}</strong> użytkownikowi <strong>{}</strong>",
                    file.Filename,
                    username
                ),
            )))
            .map_err(|err| Flash::error(Redirect::to(uri!(index)), format!("werid error in  delete_sharing: {:?}", err)))?
    } else {
        Err(Flash::error(
            Redirect::to(uri!(index)),
            format!(
                "Nie jesteś właścicielem pliku <strong>{}</strong>!",
                file.Filename
            ),
        ))
    }
}

#[post("/add_new_sharing_user?<file_id>", data = "<username>")]
async fn add_new_sharing_user(
    jar: &CookieJar<'_>,
    mut db: Connection<SQL>,
    username: Form<String>,
    file_id: i32,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    // Try to fetch the user from cookies
    let user_owner = User::get_from_cookies(&mut *db, jar)
        .await
        .ok_or_else(|| Flash::error(Redirect::to(uri!(index)), "Należy się zalogować!"))?;

    // Attempt to fetch the file from the database
    let f = File::get_one(&mut *db, file_id)
        .await
        .map_err(|_| Flash::error(Redirect::to(uri!(index)), "Nie znaleziono pliku."))?;

    let filename = &f.Filename;

    // Attempt to add the shared user
    UserFiles::add_shared_user(&mut *db, &user_owner, &f, username.clone())
        .await
        .map(|_| {
            Ok(Flash::success(
                Redirect::to(uri!(index)),
                format!(
                    "Udostępniono plik <strong>{}</strong> użytkownikowi <strong>{}</strong>",
                    filename,
                    username.into_inner() // Show the username
                ),
            ))
        })
        .map_err(|err| Flash::error(Redirect::to(uri!(index)), err))?
}

#[post("/plik", data = "<data>")]
async fn send_file(
    jar: &CookieJar<'_>,        // CookieJar for user authentication
    mut db: Connection<SQL>,    // Database connection
    content_type: &ContentType, // ContentType header of the incoming request
    data: Data<'_>,             // Incoming form data (multipart)
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    // ADDING HASH CHECK IF FILE IS ADDED PROPERLY!!!!

    // Try to fetch the user from cookies
    let user = User::get_from_cookies(&mut *db, jar)
        .await
        .ok_or_else(|| Flash::error(Redirect::to(uri!(index)), "Należy się zalogować!"))?;

    // Define options for processing the uploaded file
    let options = MultipartFormDataOptions::with_multipart_form_data_fields(vec![
        MultipartFormDataField::file("myfile")
            .size_limit(100_000_000) // 100MB max file size
            .content_type_by_string(Some(mime::STAR_STAR)) // Allow any content type
            .unwrap(),
    ]);

    // Parse the incoming multipart form data
    let multipart_form_data = MultipartFormData::parse(content_type, data, options)
        .await
        .map_err(|_| Flash::error(Redirect::to(uri!(index)), "Za duży plik! <i>(100MB)</i>"))?;

    // Retrieve the file
    let files = multipart_form_data
        .files
        .get("myfile")
        .ok_or_else(|| Flash::error(Redirect::to(uri!(index)), "Należy przesłać plik."))?;

    // Get the first uploaded file
    let file = &files[0];
    let file_path = &file.path;

    // Read the file and encode it to HEX
    let hex = std::fs::read(file_path)
        .map(|contents| HEXUPPER.encode(&contents))
        .map_err(|_| Flash::error(Redirect::to(uri!(index)), "Nie udało się odczytać pliku."))?;

    // Attempt to insert the file into the database
    File::new(
        file.file_name.as_ref().unwrap().clone(), // File name
        hex,                                      // File content (encoded)
        file.content_type.as_ref().map(|x| x.to_string()), // MIME type
    )
    .insert_for_owner(&mut *db, &user) // Insert the file associated with the user
    .await
    .map(|_| {
        Ok(Flash::success(
            Redirect::to(uri!(index)),
            format!(
                "Plik <strong>{}</strong> został przesłany!",
                file.file_name.as_ref().unwrap() // Show the file name in the message
            ),
        ))
    })
    .map_err(|e| Flash::error(Redirect::to(uri!(index)), format!("{:?}", e)))?
}

#[get("/delete/<file_id>")]
async fn delete_file(
    jar: &CookieJar<'_>,
    mut db: Connection<SQL>,
    file_id: i32,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    // Try to fetch the file, return error if not found
    let file = File::get_one(&mut *db, file_id)
        .await
        .map_err(|_| Flash::error(Redirect::to(uri!(index)), "Plik nie istnieje!"))?;

    // Try to delete the file and map success/error accordingly
    file.delete_file_from_user(&mut *db, jar)
        .await
        .map(|message| Ok(Flash::success(Redirect::to(uri!(index)), message)))
        .map_err(|mess| Flash::error(Redirect::to(uri!(index)), mess))?
}

#[get("/change_filename/<new_filename>/<file_id>")]
async fn change_filename(
    jar: &CookieJar<'_>,
    mut db: Connection<SQL>,
    new_filename: String,
    file_id: i32,
) -> Result<Redirect, Flash<Redirect>> {
    let user = User::get_from_cookies(&mut *db, jar)
        .await
        .ok_or_else(|| Flash::error(Redirect::to(uri!(index)), "Należy się zalogować!"))?;

    let mut file = user.get_file(file_id, &mut *db).await.ok_or_else(|| {
        Flash::error(Redirect::to(uri!(index)), "Nie masz dostępu do tego pliku!")
    })?;

    file.change_filename(&mut *db, jar, new_filename)
        .await
        .map(|_| Redirect::to(uri!(index)))
        .map_err(|message| Flash::error(Redirect::to(uri!(index)), message))
}

#[get("/get/<file_id>")]
async fn get_file_by_id(
    jar: &CookieJar<'_>,
    mut db: Connection<SQL>,
    file_id: i32,
) -> Result<Template, Flash<Redirect>> {
    let user = User::get_from_cookies(&mut *db, jar)
        .await
        .ok_or_else(|| Flash::error(Redirect::to(uri!(index)), "Należy się zalogować!"))?;

    let file = user.get_file(file_id, &mut *db).await.ok_or_else(|| {
        Flash::error(Redirect::to(uri!(index)), "Nie masz dostępu do tego pliku!")
    })?;

    let bytes = file.Content.as_bytes();
    let hex = HEXUPPER
        .decode(bytes)
        .map_err(|_| Flash::error(Redirect::to(uri!(index)), "Błąd dekodowania pliku!"))?;

    Ok(Template::render(
        "file",
        context! {
            mimetype: file.MimeType.unwrap(),
            data: BASE64_STANDARD.encode(&hex),
        },
    ))
}

#[post("/login", data = "<maker_user>")]
async fn index_login<'a>(
    mut db: Connection<SQL>,
    jar: &CookieJar<'_>,
    maker_user: Form<UserMaker<'_>>,
    logs: &'a State<Log>,
) -> Result<Redirect, Flash<Redirect>> {
    match maker_user.check_user_login(&mut *db).await {
        Ok(mut user) => {
            user.create_new_session(&mut *db, jar).await;
            logs.register(vec![maker_user.uname, "login", "", "1"]);
            Ok(Redirect::to(uri!(index)))
        }
        Err(err) => {
            logs.register(vec![maker_user.uname, "login", &err.get_reason(), "0"]);
            Err(Flash::new(
                Redirect::to(uri!(index)),
                &err.get_reason().to_string(),
                err.get_username(),
            ))
        }
    }
}

#[get("/logout")]
async fn index_logout(db: Connection<SQL>, jar: &CookieJar<'_>) -> Redirect {
    User::logout(db, jar).await;
    Redirect::to(uri!(index))
}

use crate::logs::Log;
use crate::users_files::UserFiles;

#[launch]
fn rocket() -> Rocket<Build> {
    let figment = rocket::Config::figment()
        .merge(("address", "0.0.0.0"))
        .merge((
            "databases.MainFrame",
            rocket_db_pools::Config {
                url: "MainFrame.db".into(),
                //url: "mssql://[USER]:[PWD]@localhost:1433/MainFrame".into(),
                min_connections: None,
                max_connections: 1024,
                connect_timeout: 5,
                idle_timeout: None,
                extensions: None,
            },
        ));
    rocket::custom(figment)
        .attach(SQL::init())
        .mount(
            "/",
            routes![
                register_new_post,
                register_new,
                delete_sharing,
                add_new_sharing_user,
                index,
                index_login,
                index_logout,
                get_file_by_id,
                send_file,
                delete_file,
                change_filename
            ],
        )
        .mount("/assets", FileServer::from(relative!("assets")))
        .attach(Template::custom(|eng| {
            eng.handlebars
                .register_helper("mod", Box::new(hbs_helpers::modulo));
        }))
        .manage(Log::new())
}
