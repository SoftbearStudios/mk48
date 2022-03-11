// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::UnixTime;
use std::fs::File;
use std::io::Read;
use x509_parser::prelude::parse_x509_pem;

pub fn certificate_expiry(certificate_file: &str) -> Result<UnixTime, String> {
    let mut f = File::open(certificate_file).map_err(|e| e.to_string())?;
    let mut cert = Vec::new();
    f.read_to_end(&mut cert).map_err(|e| e.to_string())?;
    let (_, pem) = parse_x509_pem(&cert).map_err(|e| e.to_string())?;
    let x509 = pem.parse_x509().map_err(|e| e.to_string())?;
    Ok(x509.validity().not_after.timestamp() as UnixTime)
}
