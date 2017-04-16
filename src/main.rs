#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate multipart;
extern crate rocket_contrib;
extern crate rand;
extern crate mime_guess;
extern crate glob;

use rand::{thread_rng, Rng};
use rocket_contrib::Template;
use std::path::{Path, PathBuf};
use std::fs::File;
use mime_guess::get_mime_extensions_str;
use std::io::prelude::*;
use rocket::response::NamedFile;
use glob::glob;

fn random_name(n: usize) -> String {
    thread_rng().gen_ascii_chars().take(n).collect()
}

#[get("/<img>")]
fn image(img: String) -> Option<NamedFile> {
    for entry in glob(format!("./uploads/{}.*", img).as_str()).expect("glob failed") {
        match entry {
            Ok(path) => {
                return NamedFile::open(path).ok()
            },
            Err(e) => println!("{:?}", e),
        }
    }
    None
}

#[get("/")]
fn index() -> Template {
    Template::render("index", &())
}

#[post("/", data = "<upload>")]
fn index_upload(upload: FileUpload) -> String {
    let name = random_name(8);
    let rawName = name.clone();
    let fileName = name + "." + match get_mime_extensions_str(&upload.mime) {
        None => "png",
        Some(extensions) => extensions[0],
    };

    let mut path = String::from("uploads/");
    path.push_str(fileName.as_str());
    let mut file = File::create(path).unwrap();
    file.write_all(upload.file.as_slice()).unwrap();

    format!("localhost:6969/{}", rawName)

}

#[get("/assets/<file..>")]
fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).ok()
}

#[derive(Debug)]
struct FileUpload {
    username: String,
    password: String,
    mime: String,
    file: Vec<u8>,
}

use std::io::{Cursor, Read};
use rocket::{Request, Data, Outcome};
use rocket::data::{self, FromData};
use multipart::server::Multipart;

impl FromData for FileUpload {
    type Error = ();

    fn from_data(request: &Request, data: Data) -> data::Outcome<Self, Self::Error> {
        // All of these errors should be reported
        let ct = request.headers().get_one("Content-Type").expect("no content-type");
        let idx = ct.find("boundary=").expect("no boundary");
        let boundary = &ct[(idx + "boundary=".len())..];

        let mut d = Vec::new();
        data.stream_to(&mut d).expect("Unable to read");

        let mut mp = Multipart::with_body(Cursor::new(d), boundary);

        // Custom implementation parts

        let mut username = None;
        let mut password = None;
        let mut file = None;
        let mut mime = None;

        mp.foreach_entry(|mut entry| {
            match entry.name.as_str() {
                "mime" => { 
                    let t = entry.data.as_text().expect("not text");
                    mime = Some(t.into());
                },
                "username" => {
                    let t = entry.data.as_text().expect("not text");
                    username = Some(t.into());
                },
                "password" => {
                    let t = entry.data.as_text().expect("not text");
                    password = Some(t.into());
                },
                "file" => {
                    let mut d = Vec::new();
                    let f = entry.data.as_file().expect("not file");
                    f.read_to_end(&mut d).expect("cant read");
                    file = Some(d);
                },
                other => panic!("No known key {}", other),
            }
        }).expect("Unable to iterate");

        let v = FileUpload {
            username: username.expect("username not set"),
            password: password.expect("password not set"),
            file: file.expect("file not set"),
            mime: mime.expect("mime not present"),
        };

        // End custom

        Outcome::Success(v)
    }
}

fn main() {
    rocket::ignite().mount("/", routes![image, files, index_upload, index]).launch();
}