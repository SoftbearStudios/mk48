use actix_http::KeepAlive;
use actix_tls::accept::rustls::TlsStream;
use actix_web::dev::Extensions;
use actix_web::rt::net::TcpStream;
use log::{error, info};
use std::any::Any;
use std::time::Duration;

/// Tcp Keep Alive.
pub const KEEP_ALIVE: KeepAlive = KeepAlive::Timeout(Duration::from_secs(10));

/// Tcp server shutdown timeout, in seconds.
pub const SHUTDOWN_TIMEOUT: u64 = 3;

/// Tcp connection backlog.
pub const BACKLOG: u32 = 60;

/// Returns how many connections should be allowed per server worker.
pub fn max_connections_per_worker() -> usize {
    const MAX_FILE_DESCRIPTORS: usize = 1000;
    const CLEARANCE: usize = 50;
    const MAX_CONNECTIONS: usize = MAX_FILE_DESCRIPTORS - BACKLOG as usize - CLEARANCE;
    let workers = num_cpus::get();
    let max_connections_per_worker = MAX_CONNECTIONS / workers;

    info!(
        "Server will spawn {} workers, each with up to {} connections",
        workers, max_connections_per_worker
    );

    max_connections_per_worker
}

/// Usabe as an on_connect callback, this enables TCP_NODELAY.
pub fn on_connect_enable_nodelay(conn: &dyn Any, _ext: &mut Extensions) {
    let plain = if let Some(rustls) = conn.downcast_ref::<TlsStream<TcpStream>>() {
        rustls.get_ref().0
    } else if let Some(plain) = conn.downcast_ref::<TcpStream>() {
        plain
    } else {
        debug_assert!(false);
        return;
    };

    if let Err(e) = plain.set_nodelay(true) {
        error!("error setting nodelay: {:?}", e);
    }
}
