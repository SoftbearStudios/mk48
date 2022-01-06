use rand::{thread_rng, Rng};
use std::fs;
use std::path::Path;

const AUTH_PATH: &str = "./src/auth.txt";

fn main() {
    if !Path::new(AUTH_PATH).exists() {
        fs::write(
            AUTH_PATH,
            &base64::encode(thread_rng().gen::<u128>().to_le_bytes()),
        )
        .unwrap();
    }
}
