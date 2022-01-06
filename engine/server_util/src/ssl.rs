// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_web::dev::Server;
use futures::{pin_mut, select, FutureExt};
use log::{error, info, warn};
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

    /// Returns if the certificate in the file system is newer, and the current certificate
    /// will expire imminently.
    pub fn should_renew(&self) -> bool {
        let expiry = self.available_certificate_expiry();
        expiry
            .checked_duration_since(Instant::now())
            .map(|d| d < Duration::from_secs(5 * 24 * 60 * 60))
            .unwrap_or(true)
            && expiry > self.installed_certificate_expiry + Duration::from_secs(60 * 60)
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

/// Returns when either the server has stopped (Err) or the SSL needs renewal (Ok).
pub async fn run_until_ssl_renewal<'a>(server: Server, ssl: &Option<Ssl<'a>>) -> Result<(), ()> {
    if let Some(ssl) = ssl {
        // This handle can be sent the stop command, and it will stop the original server
        // which has been moved by then.
        let server_handle = server.handle();

        let renewal = async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(12 * 60 * 60));

            // Eat first tick.
            interval.tick().await;

            loop {
                interval.tick().await;

                if ssl.should_renew() {
                    warn!("Checking if certificate can be renewed...yes");
                    // Stopping this future will trigger a restart.
                    break;
                } else {
                    info!("Checking if certificate can be renewed...no");
                }
            }
        };

        //let fused_server = (Box::new(running_server) as Box<dyn futures::Future<Output=Result<(), std::io::Error>>>);
        let fused_server = server.fuse();
        let fused_renewal = renewal.fuse();

        pin_mut!(fused_server, fused_renewal);

        select! {
            res = fused_server => {
                error!("server result: {:?}", res);
                Err(())
            },
            () = fused_renewal => {
                server_handle.stop(true).await;
                Ok(())
            }
        }
    } else {
        let _ = server.await;
        Err(())
    }
}
