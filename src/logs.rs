use chrono::prelude::*;
use std::fs::OpenOptions;
use std::io::Write;

pub struct Log {
    date: DateTime<Local>,
    filepath: std::path::PathBuf,
}

impl Log {
    pub fn new() -> Log {
        let now = Local::now();
        let day_str = now.format("%Y_%m_%d").to_string();
        let form = format!("Log{}.csv", now.format("%H_%M_%S").to_string());

        let f = Log {
            date: now,
            filepath: std::path::Path::new("logs").join(day_str).join(form),
        };

        // create parent directories log and day
        let _ = std::fs::create_dir_all(&f.filepath.parent().unwrap());

        // create(overwrite) log.csv file
        let _ = writeln!(
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(&f.filepath)
                .unwrap(),
            "Czas,Użytkownik,Powód,Opis,Status"
        );

        f
    }

    pub fn get_filename(&self) -> &str {
        &self.filepath.to_str().unwrap()
    }

    fn write(&self, text: &str) {
        let mut opt = match OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&self.filepath)
        {
            Ok(file) => file,
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
				println!("Could not locate previous log directory (deleted?). Creating new.");
                let _ = std::fs::create_dir_all(&self.filepath.parent().unwrap());

                let _ = writeln!(
                    std::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .open(&self.filepath)
                        .unwrap(),
                    "Czas,Użytkownik,Powód,Opis,Status"
                );

                match OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(&self.filepath)
                {
                    Ok(file) => file,
                    Err(_) => panic!("Even creating paths again created error!"),
                }
            }
            Err(_) => panic!(),
        };

        let _ = writeln!(opt, "{}", text);
    }

    pub fn register(&self, text: Vec<&str>) {
        let now = Local::now();

        self.write(&format!("{},{}", now.format("%H_%M_%S").to_string(), text.join(",")));
    }
}
