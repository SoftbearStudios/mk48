// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::rpc::AdminRequest;
use minicdn::EmbeddedMiniCdn;
use serde::Serialize;
use std::time::Duration;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Options {
    #[structopt(long)]
    path: String,
    #[structopt(long)]
    url: String,
    #[structopt(long)]
    no_compress: bool,
}

#[derive(Serialize)]
pub struct ParameterizedAdminRequest {
    pub auth: String,
    pub request: AdminRequest,
}

fn main() {
    let options: Options = Options::from_args();
    let cdn = if options.no_compress {
        EmbeddedMiniCdn::new(&options.path)
    } else {
        EmbeddedMiniCdn::new_compressed(&options.path)
    };

    let len = cdn.iter().count();

    if len == 0 {
        eprintln!("no files");
        std::process::exit(1);
    }

    let auth = include_str!("../../game_server/src/auth.txt").to_owned();

    let msg = ParameterizedAdminRequest {
        auth: auth.clone(),
        request: AdminRequest::SetGameClient(cdn.clone()),
    };

    let body = serde_json::to_string(&msg).unwrap();

    eprintln!("found {} files ({} bytes):", len, body.len());

    for (path, file) in cdn.iter() {
        eprintln!(
            " - \"{}\" ({} bytes uncompressed)",
            path,
            file.contents.len()
        );
    }

    eprintln!();
    eprintln!("total bytes: {}", body.len());

    eprintln!();
    eprintln!("pausing for a moment...");

    std::thread::sleep(Duration::from_secs(2));

    eprintln!("uploading...");

    let client = reqwest::blocking::ClientBuilder::new()
        .tcp_keepalive(Some(Duration::from_secs(10)))
        .pool_max_idle_per_host(0)
        .build()
        .unwrap();
    match client
        .post(options.url)
        .header("auth", auth)
        .header("content-type", "application/json")
        .body(body)
        .send()
    {
        Ok(response) => {
            let status = response.status();
            match response.text() {
                Ok(text) => {
                    println!("received: {} (code {})", text, status);
                }
                Err(e) => eprintln!("{}", e.to_string()),
            }
        }
        Err(e) => eprintln!("{}", e.to_string()),
    }
}
