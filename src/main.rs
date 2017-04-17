#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate multipart;
extern crate rocket_contrib;
extern crate rand;
extern crate mime_guess;
extern crate glob;
extern crate sha2;

use std::io::{Cursor, Read};
use rocket::{Request, Data, Outcome};
use rocket::data::{self, FromData};
use rocket::State;
use multipart::server::Multipart;
use rand::{thread_rng, Rng};
use rocket_contrib::Template;
use std::path::{Path, PathBuf};
use std::fs::File;
use mime_guess::get_mime_extensions_str;
use std::io::prelude::*;
use rocket::response::NamedFile;
use glob::glob;
use std::vec::Vec;
use std::iter::FromIterator;
use rocket::config::{self};
use sha2::{Sha512, Digest};

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
fn index_upload(upload: FileUpload, conf: State<ShareXConfig>) -> Option<String> {
    let pass = upload.password.clone();
    let pass_hash = conf.pass_hash.clone();
    if !verify(pass, pass_hash) {
        return None;
    }
    let name = random_name(8);
    let raw_name = name.clone();
    let file_name = name + "." + match get_mime_extensions_str(&upload.mime) {
        None => "png",
        Some(extensions) => extensions[0],
    };

    let mut path = String::from("uploads/");
    path.push_str(file_name.as_str());
    let mut file = File::create(path).unwrap();
    file.write_all(upload.file.as_slice()).unwrap();

    Some(format!("localhost:6969/{}", raw_name))

}

#[get("/assets/<file..>")]
fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).ok()
}

#[derive(Debug)]
struct ShareXConfig {
    pass_hash: String,
}

impl ShareXConfig {
    fn new(hash: String) -> Self {
        ShareXConfig {
            pass_hash: hash
        }
    }
}

#[derive(Debug)]
struct FileUpload {
    password: String,
    mime: String,
    file: Vec<u8>,
}

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

        let mut password = None;
        let mut file = None;
        let mut mime = None;

        mp.foreach_entry(|mut entry| {
            match entry.name.as_str() {
                "mime" => { 
                    let t = entry.data.as_text().expect("not text");
                    mime = Some(t.into());
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
            password: password.expect("password not set"),
            file: file.expect("file not set"),
            mime: mime.expect("mime not present"),
        };

        // End custom

        Outcome::Success(v)
    }
}

fn verify(clear: String, hash: String) -> bool {
    let mut hasher = Sha512::default();
    hasher.input(clear.as_bytes());
    let output = hasher.result().into_iter().map(|x| format!("{:02x}",x).to_string()).collect::<String>();
    output == hash
}

fn main() {
    let rock = rocket::ignite()
        .mount("/", routes![image, files, index_upload, index])
        .manage(ShareXConfig::new(config::active().unwrap().get_str("password_hash").unwrap().to_string()))
        .launch();
}