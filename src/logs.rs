use std::fs::OpenOptions;
use chrono::prelude::*;
use chrono::{Datelike, Timelike};
use std::io::Write;
use std::path::Path;

pub struct Log{
	filename: String
}

impl Log{
	pub fn new()->Log{
		let now = Local::now();
		let form = format!("Log{}-{}-{}+{}_{}_{}.csv", now.year(), now.month(), now.day(), now.hour(), now.minute(), now.second());
		std::fs::File::create(&form);
		let f = Log{
			filename: form
		};

		f.append(vec!["Czas","Użytkownik","Opis"]);

		f
	}

	pub fn get_filename(&self)->&str{
		self.filename.as_ref()
	}

	pub fn append(&self, text: Vec<&str>){
		let mut file  = OpenOptions::new().create(true).write(true).append(true).open(self.filename.clone()).unwrap();
		writeln!(file,"{}", text.join(","));
	}

	pub fn register(&self, text: Vec<&str>){
		let now = Local::now();
		let h = format!("{}:{}:{}", now.hour(), now.minute(), now.second());
		let mut file  = OpenOptions::new().create(true).write(true).append(true).open(self.filename.clone()).unwrap();
		writeln!(file,"{},{}", h,text.join(","));
	}
}