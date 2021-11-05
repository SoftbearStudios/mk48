// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use log::warn;
use rustls::server::{NoClientAuth, ServerConfig};
use rustls_pemfile;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Read};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use x509_parser::pem::parse_x509_pem;

pub struct Ssl<'a> {
    certificate_file: &'a str,
    private_key_file: &'a str,
    installed_certificate_expiry: Instant,
}

impl<'a> Ssl<'a> {
    pub fn new(certificate_file: &'a str, private_key_file: &'a str) -> Result<Self, &'static str> {
        if !fs::metadata(certificate_file).is_ok() {
            return Err("certificate file missing");
        }
        if !fs::metadata(private_key_file).is_ok() {
            return Err("private key file missing");
        }
        let ret = Self {
            certificate_file,
            private_key_file,
            installed_certificate_expiry: Self::certificate_expiry(certificate_file),
        };
        if !ret.is_valid() {
            return Err("certificate expired");
        }
        Ok(ret)
    }

    /// Returns true if and only if the filesystem contains the certificate and private key files.
    fn keys_present(&self) -> bool {
        fs::metadata(self.certificate_file).is_ok() && fs::metadata(self.private_key_file).is_ok()
    }

    /// Returns if the certificate in the file system is newer (by at least a day).
    pub fn can_renew(&self) -> bool {
        self.available_certificate_expiry()
            > self.installed_certificate_expiry + Duration::from_secs(24 * 60 * 60)
    }

    /// Call when the renewed certificate is introduced.
    pub fn set_renewed(&mut self) {
        self.installed_certificate_expiry = self.available_certificate_expiry();
    }

    /// Returns true if and only if the filesystem contains non-expired certificate and private key.
    pub fn is_valid(&self) -> bool {
        if !self.keys_present() {
            return false;
        }
        let expiry = self.available_certificate_expiry();
        let now = Instant::now();

        if expiry > now {
            let expires_in = expiry.duration_since(now);
            warn!("Certificate expires in {:?}", expires_in);
        }

        now < expiry
    }

    /// Gets a rustls configuration for the certificates currently in the file system.
    pub fn rustls_config(&self) -> ServerConfig {
        let cert_file = File::open(self.certificate_file).unwrap();
        let mut cert_reader = BufReader::new(cert_file);

        let priv_file = File::open(self.private_key_file).unwrap();
        let mut priv_reader = BufReader::new(priv_file);

        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_client_cert_verifier(NoClientAuth::new())
            .with_single_cert(
                rustls_pemfile::certs(&mut cert_reader)
                    .unwrap()
                    .into_iter()
                    .map(|v| rustls::Certificate(v))
                    .collect(),
                rustls::PrivateKey(
                    rustls_pemfile::pkcs8_private_keys(&mut priv_reader)
                        .unwrap()
                        .into_iter()
                        .next()
                        .unwrap(),
                ),
            )
            .unwrap();

        config
    }

    fn certificate_expiry(certificate_file: &str) -> Instant {
        let mut f = File::open(certificate_file).unwrap();
        let mut cert = Vec::new();
        f.read_to_end(&mut cert).unwrap();
        let (_, pem) = parse_x509_pem(&cert).ok().unwrap();
        let x509 = pem.parse_x509().unwrap();
        let not_after = x509.validity().not_after.timestamp() as u64;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let delta = not_after - now;

        Instant::now() + Duration::from_secs(delta)
    }

    pub fn available_certificate_expiry(&self) -> Instant {
        Self::certificate_expiry(self.certificate_file)
    }
}
