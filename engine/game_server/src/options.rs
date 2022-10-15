// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use core_protocol::id::RegionId;
use log::{warn, LevelFilter};
use std::net::IpAddr;
use std::num::NonZeroU64;
use structopt::StructOpt;

/// Server options, to be specified as arguments.
#[derive(Debug, StructOpt)]
pub struct Options {
    /// Minimum number of bots.
    #[structopt(long)]
    pub min_bots: Option<usize>,
    /// Maximum number of bots.
    #[structopt(long)]
    pub max_bots: Option<usize>,
    /// This percent of real players will help determine number of bots.
    #[structopt(long)]
    pub bot_percent: Option<usize>,
    /// Log incoming HTTP requests
    #[cfg_attr(debug_assertions, structopt(long, default_value = "warn"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "error"))]
    pub debug_http: LevelFilter,
    /// Log game diagnostics
    #[cfg_attr(debug_assertions, structopt(long, default_value = "info"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "error"))]
    pub debug_game: LevelFilter,
    /// Log core diagnostics
    #[cfg_attr(debug_assertions, structopt(long, default_value = "info"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "error"))]
    pub debug_core: LevelFilter,
    /// Log socket diagnostics
    #[cfg_attr(debug_assertions, structopt(long, default_value = "warn"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "error"))]
    pub debug_sockets: LevelFilter,
    /// Log watchdog diagnostics
    #[cfg_attr(debug_assertions, structopt(long, default_value = "info"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "warn"))]
    pub debug_watchdog: LevelFilter,
    /// Log chats here
    #[structopt(long)]
    pub chat_log: Option<String>,
    /// Log client traces here
    #[structopt(long)]
    pub trace_log: Option<String>,
    /// Persist admin config here.
    #[structopt(long)]
    pub admin_config_file: Option<String>,
    /// Linode personal access token for DNS configuration.
    #[structopt(long)]
    pub linode_personal_access_token: Option<String>,
    /// Discord application client id (public).
    #[structopt(long, default_value = "996616106431225958")]
    pub discord_client_id: String,
    /// Discord application client secret.
    #[structopt(long)]
    pub discord_client_secret: Option<String>,
    /// Discord bot token.
    #[structopt(long)]
    pub discord_bot_token: Option<String>,
    /// Discord guild (server) id.
    #[structopt(long, default_value = "847143438939717663")]
    pub discord_guild_id: NonZeroU64,
    /// Don't write to the database.
    #[structopt(long)]
    pub database_read_only: bool,
    /// Server id.
    #[structopt(long, default_value = "0")]
    pub server_id: u8,
    #[structopt(long)]
    /// Override the server ip (currently used to detect the region).
    pub ip_address: Option<IpAddr>,
    #[structopt(long)]
    pub http_port: Option<u16>,
    /// Override the region id.
    #[structopt(long)]
    pub region_id: Option<RegionId>,
    /// Domain (without server id prepended).
    #[allow(dead_code)]
    #[structopt(long)]
    pub domain: Option<String>,
    /// Certificate chain path.
    #[structopt(long)]
    pub certificate_path: Option<String>,
    /// Private key path.
    #[structopt(long)]
    pub private_key_path: Option<String>,
    /// HTTP request bandwidth limiting (in bytes per second).
    #[structopt(long, default_value = "500000")]
    pub http_bandwidth_limit: u32,
    /// HTTP request rate limiting burst (in bytes).
    ///
    /// Implicit minimum is double the total size of the client static files.
    #[cfg_attr(debug_assertions, structopt(long, default_value = "4294967294"))]
    #[cfg_attr(not(debug_assertions), structopt(long, default_value = "10000000"))]
    pub http_bandwidth_burst: u32,
    /// Client authenticate rate limiting period (in seconds).
    #[structopt(long, default_value = "30")]
    pub client_authenticate_rate_limit: u64,
    /// Client authenticate rate limiting burst.
    #[structopt(long, default_value = "16")]
    pub client_authenticate_burst: u32,
}

impl Options {
    pub(crate) fn bandwidth_burst(&self, static_size: usize) -> u32 {
        let bandwidth_burst = self.http_bandwidth_burst.max(static_size as u32 * 2);

        if bandwidth_burst > self.http_bandwidth_burst {
            warn!(
                "Using increased bandwidth burst of {} to account for client size.",
                bandwidth_burst
            );
        }

        bandwidth_burst
    }

    pub(crate) const STANDARD_HTTP_PORT: u16 = 80;
    pub(crate) const STANDARD_HTTPS_PORT: u16 = 443;

    pub(crate) fn http_and_https_ports(&self) -> (u16, u16) {
        #[cfg(unix)]
        let default_ports = if nix::unistd::Uid::effective().is_root() {
            (Self::STANDARD_HTTP_PORT, Self::STANDARD_HTTPS_PORT)
        } else {
            (8080, 8443)
        };

        #[cfg(not(unix))]
        let default_ports = (Self::STANDARD_HTTP_PORT, Self::STANDARD_HTTPS_PORT);

        let ports = (self.http_port.unwrap_or(default_ports.0), default_ports.1);
        log::info!("HTTP port is {}", ports.0);
        ports
    }
}
