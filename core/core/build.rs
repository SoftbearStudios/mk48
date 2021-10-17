use rand::{thread_rng, Rng};
use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;

const AUTH_PATH: &str = "./src/auth.txt";
const REGEXES_PATH: &str = "./src/regexes.yaml";
const REGEXES_URL: &str =
    "https://raw.githubusercontent.com/ua-parser/uap-core/master/regexes.yaml";

fn main() {
    if !Path::new(REGEXES_PATH).exists() {
        let mut resp = reqwest::blocking::get(REGEXES_URL).expect("request failed");
        let mut out = File::create(REGEXES_PATH).expect("failed to create file");
        io::copy(&mut resp, &mut out).expect("failed to copy content");
    }

    if !Path::new(AUTH_PATH).exists() {
        fs::write(
            AUTH_PATH,
            &base64::encode(thread_rng().gen::<u128>().to_le_bytes()),
        )
        .unwrap();
    }
}
