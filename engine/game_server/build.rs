// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use rand::{thread_rng, Rng};
use std::fs;
use std::path::Path;

const AUTH_PATH: &str = "./src/auth.txt";
const CERTIFICATE_PATH: &str = "./src/certificate.pem";
const PRIVATE_KEY_PATH: &str = "./src/private_key.pem";

fn main() {
    if !Path::new(AUTH_PATH).exists() {
        fs::write(
            AUTH_PATH,
            &base64::encode(thread_rng().gen::<u128>().to_le_bytes()),
        )
        .unwrap();
    }

    if !(Path::new(CERTIFICATE_PATH).exists() && Path::new(PRIVATE_KEY_PATH).exists()) {
        let cert = rcgen::generate_simple_self_signed(Vec::new()).unwrap();
        fs::write(CERTIFICATE_PATH, cert.serialize_pem().unwrap().into_bytes()).unwrap();
        fs::write(
            PRIVATE_KEY_PATH,
            cert.serialize_private_key_pem().into_bytes(),
        )
        .unwrap();
    }
}
