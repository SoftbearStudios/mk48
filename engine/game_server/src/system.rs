// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game_service::GameArenaService;
use crate::infrastructure::Infrastructure;
use crate::status::StatusRepo;
use crate::util::diff_small_n;
use actix::fut::wrap_future;
use actix::{
    ActorFutureExt, ActorStreamExt, Context as ActorContext, ContextFutureSpawner, Handler,
    Message, WrapFuture, WrapStream,
};
use core_protocol::dto::ServerDto;
use core_protocol::id::{InvitationId, RegionId, ServerId};
use core_protocol::rpc::{StatusResponse, SystemResponse, SystemUpdate};
use db_ip::{include_region_database, DbIpDatabase, Region};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::{error, info, warn};
use rand::prelude::IteratorRandom;
use rand::{thread_rng, Rng};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::redirect::Policy;
use reqwest::Client;
use serde::Deserialize;
use server_util::cloud::{Cloud, DnsUpdate};
use server_util::rate_limiter::RateLimiter;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Monitors web servers and changes DNS to recover from servers going offline.
///
/// System, in this case, refers to a distributed system of multiple servers.
pub struct SystemRepo<G: GameArenaService> {
    domain: Arc<String>,
    /// All servers on the domain.
    pub(crate) servers: HashMap<ServerId, ServerData>,
    /// For diffing. Always sorted in ascending order of [`ServerId`].
    previous: Arc<[ServerDto]>,
    cloud: &'static dyn Cloud,
    redirect_server_id: &'static AtomicU8,
    update_rate_limiter: RateLimiter,
    _spooky: PhantomData<G>,
}

#[derive(Debug)]
pub(crate) struct ServerData {
    /// Public IP address of server.
    pub ip: IpAddr,
    /// Best guess for [`RegionId`] of server.
    pub region_id: Option<RegionId>,
    /// Last known status of whether server was hosting home page.
    pub home: bool,
    /// Network round-trip-time.
    pub rtt: Duration,
    /// Last known status of server.
    pub status: ServerStatus,
}

impl ServerData {
    /// How many tries to connect to a server in the same region before assuming dead.
    const TRIES_SAME_REGION: u8 = 2;
    /// How many tries to connect to a server in a different region before assuming dead.
    const TRIES_DIFFERENT_REGION: u8 = 3;

    /// How many tries (from the given region) before assuming dead.
    pub fn tries(&self, with_respect_to: Option<RegionId>) -> u8 {
        if self.region_id == with_respect_to {
            Self::TRIES_SAME_REGION
        } else {
            Self::TRIES_DIFFERENT_REGION
        }
    }

    /// Returns [`true`] iff the server will be considered dead after one more failed try.
    pub fn is_dying(&self, with_respect_to: Option<RegionId>) -> bool {
        match &self.status {
            ServerStatus::Unreachable { tries } | ServerStatus::Unhealthy { tries, .. } => {
                *tries >= self.tries(with_respect_to).saturating_sub(1)
            }
            ServerStatus::Healthy { .. } | ServerStatus::Incompatible => false,
        }
    }
}

#[derive(Debug)]
pub(crate) enum ServerStatus {
    /// Server could not be reached after a certain number of tries.
    Unreachable { tries: u8 },
    /// Server is unhealthy for a certain number of consecutive tries.
    Unhealthy {
        tries: u8,
        advertisement: ServerAdvertisement,
    },
    /// Server status reporting is reachable but incompatible, no judgement about health.
    Incompatible,
    /// Server is healthy, and self-reported the following details.
    Healthy {
        advertisement: ServerAdvertisement,
        /// We only accept dying server ids from a healthy server.
        dying_server_ids: Vec<ServerId>,
    },
}

/// Fields that a healthy/unhealthy server may advertise about itself.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ServerAdvertisement {
    pub(crate) redirect_server_id: Option<ServerId>,
    pub(crate) client_hash: Option<u64>,
    pub(crate) player_count: Option<u32>,
}

impl ServerStatus {
    pub(crate) fn advertisement(&self) -> Option<&ServerAdvertisement> {
        match self {
            ServerStatus::Healthy { advertisement, .. }
            | ServerStatus::Unhealthy { advertisement, .. } => Some(advertisement),
            _ => None,
        }
    }

    /// Returns the client hash advertised by this server, if it is known.
    pub(crate) fn client_hash(&self) -> Option<u64> {
        self.advertisement().and_then(|ad| ad.client_hash)
    }

    /// Returns the player count advertised by this server, if it is known.
    pub(crate) fn player_count(&self) -> Option<u32> {
        self.advertisement().and_then(|ad| ad.player_count)
    }
}

struct PingResult {
    server_id: ServerId,
    ip: IpAddr,
    home: bool,
    rtt: Duration,
    status: PingResultStatus,
}

enum PingResultStatus {
    /// Unresponsive to ping.
    Unreachable,
    /// Self-reported unhealthy.
    Unhealthy {
        region_id: Option<RegionId>,
        advertisement: ServerAdvertisement,
    },
    /// Protocol error.
    Incompatible,
    /// Self-reported healthy.
    Healthy {
        region_id: Option<RegionId>,
        advertisement: ServerAdvertisement,
        dying_server_ids: Vec<ServerId>,
    },
}

impl<G: GameArenaService> SystemRepo<G> {
    /// "" is the real home. This can be set to an alternate, inconsequential, value for testing, like "home"
    #[cfg(debug_assertions)]
    const HOME: &'static str = "home";
    #[cfg(not(debug_assertions))]
    const HOME: &'static str = "";

    /// How often to ping other servers.
    const RATE: Duration = Duration::from_secs(50);

    /// How long to wait before timing out ping.
    const PING_TIMEOUT: Duration = Duration::from_secs(16);

    /// If a server that does not report its client hash is considered compatible by default.
    const MISSING_HASH_IS_COMPATIBLE: bool = false;

    pub fn new(
        cloud: Box<dyn Cloud>,
        domain: String,
        redirect_server_id: &'static AtomicU8,
    ) -> Self {
        Self {
            // Only happens once, helps async code later.
            cloud: Box::leak(cloud),
            domain: Arc::new(domain),
            redirect_server_id,
            servers: HashMap::new(),
            previous: Vec::new().into(),
            update_rate_limiter: RateLimiter::new(Self::RATE, 0),
            _spooky: PhantomData,
        }
    }

    /// Get the current actual redirect.
    /*
    pub(crate) fn get_redirect(&self) -> Option<ServerId> {
        ServerId::new(self.redirect_server_id.load(Ordering::Relaxed))
    }
     */

    /// Set the actual redirect immediately.
    pub(crate) fn set_redirect(&mut self, server_id: Option<ServerId>) {
        self.redirect_server_id.store(
            server_id.map(|id| id.0.get()).unwrap_or(0),
            Ordering::Relaxed,
        );
    }

    pub(crate) fn initializer(&self) -> Option<SystemUpdate> {
        (!self.previous.is_empty()).then(|| SystemUpdate::Added(Arc::clone(&self.previous)))
    }

    /// Compute [`ServerDto`]'s for normal players.
    fn compute_dtos(&self, status: &StatusRepo) -> Vec<ServerDto> {
        self.servers
            .iter()
            .filter_map(|(&server_id, server)| {
                if let &ServerStatus::Healthy {
                    advertisement:
                        ServerAdvertisement {
                            redirect_server_id,
                            client_hash,
                            player_count,
                        },
                    ..
                } = &server.status
                {
                    if client_hash
                        .map(|hash| hash == status.client_hash)
                        .unwrap_or(Self::MISSING_HASH_IS_COMPATIBLE)
                        && redirect_server_id.is_none()
                    {
                        if let Some((region_id, player_count)) =
                            server.region_id.zip(player_count.or(Some(0)))
                        {
                            return Some(ServerDto {
                                server_id,
                                region_id,
                                player_count,
                            });
                        }
                    }
                }
                None
            })
            .collect()
    }

    pub(crate) fn delta(
        &mut self,
        status: &StatusRepo,
    ) -> Option<(Arc<[ServerDto]>, Arc<[ServerId]>)> {
        let mut current_servers = self.compute_dtos(status);

        if let Some((added, removed)) =
            diff_small_n(&self.previous, &current_servers, |dto| dto.server_id)
        {
            current_servers.sort_unstable();
            self.previous = current_servers.into();
            Some((added.into(), removed.into()))
        } else {
            None
        }
    }

    pub(crate) fn update(
        infrastructure: &mut Infrastructure<G>,
        ctx: &mut ActorContext<Infrastructure<G>>,
    ) {
        let system = unwrap_or_return!(infrastructure.system.as_mut());

        if system.update_rate_limiter.should_limit_rate() {
            return;
        }

        let cloud = system.cloud;
        let domain_clone = Arc::clone(&system.domain);

        Box::pin(async move { cloud.read_dns(&domain_clone).await })
            .into_actor(infrastructure)
            .map(Self::update_step_2)
            .spawn(ctx);
    }

    fn update_step_2(
        result: Result<HashMap<String, Vec<IpAddr>>, &'static str>,
        infrastructure: &mut Infrastructure<G>,
        ctx: &mut ActorContext<Infrastructure<G>>,
    ) {
        let system = infrastructure.system.as_mut().unwrap();

        let mut records = match result {
            Ok(res) => res,
            Err(e) => {
                error!("watchdog cloud error: {:?}", e);
                return;
            }
        };

        let mut default_headers = HeaderMap::new();

        default_headers.insert(
            reqwest::header::CONNECTION,
            HeaderValue::from_str("close").unwrap(),
        );

        let client = match Client::builder()
            .timeout(Self::PING_TIMEOUT)
            .http1_only()
            .default_headers(default_headers)
            .redirect(Policy::none())
            .build()
        {
            Ok(client) => client,
            Err(_) => return,
        };

        let home_ip_addresses: HashSet<IpAddr> = records
            .remove(Self::HOME)
            .unwrap_or_default()
            .into_iter()
            .collect();

        // Delete from this when we see a server in DNS results. When done processing DNS results,
        // this will store what should be expired.
        let mut expire: HashSet<ServerId> = system.servers.keys().copied().collect();

        let now = Instant::now();

        let pings: FuturesUnordered<_> = records
            .into_iter()
            .filter_map(|(sub_domain, ip_addresses)| {
                if ip_addresses.len() != 1 {
                    None
                } else if let Some(server_id) =
                    sub_domain.parse::<u8>().ok().and_then(ServerId::new)
                {
                    Some((server_id, ip_addresses[0]))
                } else {
                    None
                }
            })
            .filter_map(|(server_id, ip)| {
                expire.remove(&server_id);

                let home = home_ip_addresses.contains(&ip);

                let client = client.clone();
                let request = client
                    .get(format!(
                        "https://{}.{}/status.json",
                        server_id.0, system.domain
                    ))
                    .build()
                    .ok()?;

                Some(async move {
                    let response = match client.execute(request).await {
                        Ok(response) => response,
                        Err(e) => {
                            warn!("watchdog send request error with {:?}: {}", server_id, e);
                            return PingResult {
                                server_id,
                                ip,
                                home,
                                rtt: now.elapsed(),
                                status: PingResultStatus::Unreachable,
                            };
                        }
                    };

                    let body = match response.text().await {
                        Ok(b) => b,
                        Err(e) => {
                            warn!("watchdog response error with {:?}: {}", server_id, e);
                            return PingResult {
                                server_id,
                                ip,
                                home,
                                rtt: now.elapsed(),
                                status: PingResultStatus::Unreachable,
                            };
                        }
                    };

                    #[derive(Deserialize)]
                    struct Response {
                        #[serde(rename = "StatusRequested")]
                        status: StatusResponse,
                    }

                    let status = match serde_json::from_slice::<Response>(body.as_ref())
                        .map(|r| r.status)
                        .or(serde_json::from_slice::<StatusResponse>(body.as_ref()))
                    {
                        Ok(status) => {
                            let advertisement = ServerAdvertisement {
                                redirect_server_id: status.redirect_server_id,
                                client_hash: status.client_hash,
                                player_count: status.player_count,
                            };
                            if status.healthy {
                                info!("watchdog {:?} is healthy", server_id);
                                PingResultStatus::Healthy {
                                    region_id: status.region_id,
                                    advertisement,
                                    dying_server_ids: status.dying_server_ids,
                                }
                            } else {
                                warn!("watchdog {:?} is unhealthy", server_id);
                                PingResultStatus::Unhealthy {
                                    region_id: status.region_id,
                                    advertisement,
                                }
                            }
                        }
                        Err(e) => {
                            warn!("watchdog {:?} is incompatible: {:?}", server_id, e);
                            PingResultStatus::Incompatible
                        }
                    };

                    PingResult {
                        server_id,
                        ip,
                        home,
                        rtt: now.elapsed(),
                        status,
                    }
                })
            })
            .collect();

        for server_id in expire {
            warn!("forgetting {:?}", server_id);
            system.servers.remove(&server_id);
        }

        pings
            .into_actor(infrastructure)
            .map(Self::update_step_3)
            .finish()
            .map(Self::update_step_4)
            .spawn(ctx);
    }

    fn update_step_3(
        ping_result: PingResult,
        infrastructure: &mut Infrastructure<G>,
        _ctx: &mut ActorContext<Infrastructure<G>>,
    ) {
        let server = infrastructure
            .system
            .as_mut()
            .unwrap()
            .servers
            .entry(ping_result.server_id)
            .or_insert(ServerData {
                ip: ping_result.ip,
                region_id: Self::ip_to_region_id(ping_result.ip),
                home: ping_result.home,
                rtt: ping_result.rtt,
                // Will be overwritten.
                status: ServerStatus::Incompatible,
            });

        server.ip = ping_result.ip;
        server.home = ping_result.home;
        server.rtt = ping_result.rtt;
        server.status = match ping_result.status {
            PingResultStatus::Unreachable => {
                let tries = match &server.status {
                    ServerStatus::Unreachable { tries } => *tries,
                    _ => 0,
                }
                .saturating_add(1);
                ServerStatus::Unreachable { tries }
            }
            PingResultStatus::Unhealthy {
                region_id,
                advertisement,
            } => {
                if let Some(region_id) = region_id {
                    // Take the other server's word for its region.
                    server.region_id = Some(region_id);
                }

                let tries = match &server.status {
                    ServerStatus::Unhealthy { tries, .. } => *tries,
                    _ => 0,
                }
                .saturating_add(1);
                ServerStatus::Unhealthy {
                    tries,
                    advertisement,
                }
            }
            PingResultStatus::Incompatible => ServerStatus::Incompatible,
            PingResultStatus::Healthy {
                region_id,
                advertisement,
                dying_server_ids,
            } => {
                if let Some(region_id) = region_id {
                    // Take the other server's word for its region.
                    server.region_id = Some(region_id);
                }
                ServerStatus::Healthy {
                    advertisement,
                    dying_server_ids,
                }
            }
        };
    }

    fn update_step_4(
        _output: (),
        infrastructure: &mut Infrastructure<G>,
        ctx: &mut ActorContext<Infrastructure<G>>,
    ) {
        let infrastructure_region_id = infrastructure.region_id;
        let system = unwrap_or_return!(infrastructure.system.as_mut());

        // Update redirect based on whether desired server is ok.
        system.set_redirect(infrastructure.admin.redirect_server_id_preference.filter(
            |server_id| {
                let ok = system
                    .servers
                    .get(server_id)
                    .map(|s| {
                        matches!(
                            s.status,
                            ServerStatus::Healthy { .. } | ServerStatus::Incompatible
                        )
                    })
                    .unwrap_or(false);
                if !ok {
                    warn!(
                        "ignoring redirect {:?} due to unhealthy/unreachable status.",
                        server_id
                    );
                }
                ok
            },
        ));

        let mut home = Vec::new();
        let mut alive = Vec::new();
        // How many other servers think this server is dying.
        let mut dying_corroboration = HashMap::<ServerId, u32>::new();

        for (&server_id, server) in &system.servers {
            if server.home {
                home.push(server_id);
            }
            if matches!(
                server.status,
                ServerStatus::Healthy { .. } | ServerStatus::Incompatible
            ) {
                alive.push(server_id);
            }
            if let ServerStatus::Healthy {
                dying_server_ids, ..
            } = &server.status
            {
                if infrastructure.server_id != Some(server_id) {
                    // We trust the corroboration of a server in a different region than ourselves.
                    let other_region_agrees = infrastructure.region_id != server.region_id;

                    for &dying_server_id in dying_server_ids {
                        let dying_server_region_id = system
                            .servers
                            .get(&dying_server_id)
                            .and_then(|s| s.region_id);

                        // We trust the corroboration of a server in the same reason as the server
                        // that is supposedly dying.
                        let same_region_agrees = dying_server_region_id.is_some()
                            && dying_server_region_id == server.region_id;

                        // The only servers this excludes are servers in our region in the case that
                        // the dying server is in a different region. For example, two servers
                        // in NorthAmerica can't terminate a server in SouthAmerica without
                        // corroboration from a server outside of NorthAmerica.
                        if same_region_agrees || other_region_agrees {
                            *dying_corroboration.entry(dying_server_id).or_default() += 1;
                        }
                    }
                }
            }
        }

        if alive.is_empty() {
            error!("there are no alive servers to promote");
            return;
        }

        // Let the network warm up before terminating other servers.
        if infrastructure.status.uptime() < Self::RATE * 5 {
            info!("watchdog DNS termination is currently disengaged, pending dry runs");
        } else {
            home.drain_filter(|&mut server_id| {
                let server = system.servers.get_mut(&server_id).unwrap();

                let corroboration = dying_corroboration
                    .get(&server_id)
                    .copied()
                    .unwrap_or_default();

                match &server.status {
                    ServerStatus::Unreachable { tries } | ServerStatus::Unhealthy { tries, .. } => {
                        debug_assert!(!alive.contains(&server_id));
                        let tries_remaining = server.tries(infrastructure_region_id).saturating_sub(*tries);
                        if tries_remaining > 0 {
                            // Give another chance.
                            warn!("waiting {} more tries to remove dead {:?} from home, and {} servers agree so far", tries_remaining, server_id, corroboration);
                            return false;
                        }
                    }
                    ServerStatus::Healthy { .. } | ServerStatus::Incompatible => {
                        debug_assert!(alive.contains(&server_id));
                        return false;
                    }
                }

                if corroboration == 0 {
                    warn!("ready to remove {:?} but only {} servers agree", server_id, corroboration);
                    return false;
                }

                warn!(
                    "removing dead {:?} from home ({} other servers agree)",
                    server_id, corroboration
                );

                server.home = false;

                let cloud = system.cloud;
                let domain_clone = Arc::clone(&system.domain);
                let ip = server.ip;
                wrap_future::<_, Infrastructure<G>>(async move {
                    cloud
                        .update_dns(&domain_clone, Self::HOME, DnsUpdate::Remove(ip))
                        .await
                })
                .map(move |result, _act, _ctx| match result {
                    Ok(_) => warn!("removed {:?} from home", server_id),
                    Err(e) => error!("error removing {:?} from home: {}", server_id, e),
                })
                .spawn(ctx);

                return true;
            });
        }

        if home.is_empty() {
            alive.sort_unstable();
            if let Some(&alive_server_id) = alive.get(0) {
                let alive_server = system.servers.get_mut(&alive_server_id).unwrap();
                alive_server.home = true;
                let cloud = system.cloud;
                let domain_clone = Arc::clone(&system.domain);
                let ip = alive_server.ip;
                wrap_future::<_, Infrastructure<G>>(async move {
                    cloud
                        .update_dns(&domain_clone, Self::HOME, DnsUpdate::Add(ip))
                        .await
                })
                .map(move |result, _act, _ctx| match result {
                    Ok(_) => warn!("promoted {:?} to home", alive_server_id),
                    Err(e) => error!("error promoting {:?} to home: {}", alive_server_id, e),
                })
                .spawn(ctx);
            }
        } else {
            info!(
                "the following alive servers are hosting the homepage: {:?}",
                home
            );
        }
    }

    pub fn ip_to_region_id(ip: IpAddr) -> Option<RegionId> {
        lazy_static::lazy_static! {
            static ref DB_IP: DbIpDatabase<Region> = include_region_database!();
        }

        /// Convert from [`db_ip::Region`] to [`core_protocol::id::RegionId`].
        /// The mapping is one-to-one, since the types mirror each other.
        fn region_to_region_id(region: Region) -> RegionId {
            match region {
                Region::Africa => RegionId::Africa,
                Region::Asia => RegionId::Asia,
                Region::Europe => RegionId::Europe,
                Region::NorthAmerica => RegionId::NorthAmerica,
                Region::Oceania => RegionId::Oceania,
                Region::SouthAmerica => RegionId::SouthAmerica,
            }
        }

        DB_IP.get(&ip).map(region_to_region_id)
    }

    /// Gets public ip by consulting various 3rd party APIs.
    pub async fn get_own_public_ip() -> Option<IpAddr> {
        let mut default_headers = HeaderMap::new();

        default_headers.insert(
            reqwest::header::CONNECTION,
            HeaderValue::from_str("close").unwrap(),
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(1))
            .http1_only()
            .default_headers(default_headers)
            .build()
            .ok()?;

        let checkers = [
            "https://v4.ident.me/",
            "https://v4.tnedi.me/",
            "https://ipecho.net/plain",
            "https://ifconfig.me/ip",
            "https://icanhazip.com/",
            "https://ipinfo.io/ip",
            "https://api.ipify.org/",
        ];

        let mut checks: FuturesUnordered<_> = checkers
            .iter()
            .map(move |&checker| {
                let client = client.clone();
                let request_result = client.get(checker).build();

                async move {
                    let request = request_result.ok()?;
                    let fut = client.execute(request);

                    let response = match fut.await {
                        Ok(response) => response,
                        Err(e) => {
                            info!("checker {} returned {:?}", checker, e);
                            return None;
                        }
                    };

                    let string = match response.text().await {
                        Ok(string) => string,
                        Err(e) => {
                            info!("checker {} returned {:?}", checker, e);
                            return None;
                        }
                    };

                    match IpAddr::from_str(string.trim()) {
                        Ok(ip) => Some(ip),
                        Err(e) => {
                            info!("checker {} returned {:?}", checker, e);
                            None
                        }
                    }
                }
            })
            .collect();

        // We pick the most common API response.
        let mut guesses = HashMap::new();
        let mut max = 0;
        let mut arg_max = None;

        while let Some(check) = checks.next().await {
            if let Some(ip_address) = check {
                let entry = guesses.entry(ip_address).or_insert(0);
                *entry += 1;
                if *entry > max {
                    max = *entry;
                    arg_max = Some(ip_address);
                }
            }
        }

        info!(
            "predicting {:?} for ip (confirmed by {} of {} third parties)",
            arg_max,
            max,
            checkers.len()
        );

        arg_max
    }

    /// Iterates available servers, their absolute priorities, and player counts, in an undefined
    /// order.
    fn iter_server_priorities(
        system: &Option<SystemRepo<G>>,
        requested_server_id: Option<ServerId>,
        invitation_server_id: Option<ServerId>,
        ideal_region_id: Option<RegionId>,
    ) -> impl Iterator<Item = (ServerId, i8, u32)> + '_ {
        system
            .iter()
            .flat_map(|system| system.previous.iter())
            .map(move |server| {
                let mut priority = 0;

                if let Some(ideal_region_id) = ideal_region_id {
                    priority = ideal_region_id.distance(server.region_id) as i8;
                }

                if Some(server.server_id) == requested_server_id {
                    priority = -1;
                }

                if Some(server.server_id) == invitation_server_id {
                    priority = -2;
                }

                (server.server_id, priority, server.player_count)
            })
    }
}

/// Asks the server about the distributed system of servers.
#[derive(Message)]
#[rtype(result = "SystemResponse")]
pub struct SystemRequest {
    /// The IP address of the client.
    pub(crate) ip: IpAddr,
    /// [`ServerId`] preference.
    pub(crate) server_id: Option<ServerId>,
    /// [`RegionId`] preference.
    pub(crate) region_id: Option<RegionId>,
    /// [`InvitationId`] server preference.
    pub(crate) invitation_id: Option<InvitationId>,
}

/// Reports whether infrastructure is healthy (hardware and actor are running properly).
impl<G: GameArenaService> Handler<SystemRequest> for Infrastructure<G> {
    type Result = SystemResponse;

    fn handle(&mut self, request: SystemRequest, _: &mut Self::Context) -> Self::Result {
        let invitation_server_id = request.invitation_id.and_then(|id| id.server_id());
        let ideal_region_id = request
            .region_id
            .or_else(|| SystemRepo::<G>::ip_to_region_id(request.ip));

        let ideal_server_id = SystemRepo::iter_server_priorities(
            &self.system,
            request.server_id,
            invitation_server_id,
            ideal_region_id,
        )
        .min_by_key(|&(_, priority, player_count)| {
            (
                priority,
                if self.admin.distribute_load {
                    player_count
                } else {
                    0
                },
            )
        })
        .map(
            |(ideal_server_id, ideal_server_priority, ideal_server_player_count)| {
                if self.admin.distribute_load {
                    let mut rng = thread_rng();

                    // Prime the RNG a bit.
                    let use_player_count = rng.gen::<bool>();
                    rng.gen::<u64>();

                    let result = SystemRepo::iter_server_priorities(
                        &self.system,
                        request.server_id,
                        invitation_server_id,
                        ideal_region_id,
                    )
                    .filter(|&(_, priority, player_count)| {
                        priority == ideal_server_priority
                            && (!use_player_count || player_count == ideal_server_player_count)
                    })
                    .map(|(server_id, _, _)| server_id)
                    .choose(&mut rng);

                    if let Some(result) = result {
                        result
                    } else {
                        debug_assert!(false, "server id rug pull");
                        ideal_server_id
                    }
                } else {
                    ideal_server_id
                }
            },
        );

        SystemResponse {
            server_id: ideal_server_id.or(self.server_id),
            //servers: self.system.as_ref().map(|system| Arc::clone(system.previous)).unwrap_or_else(|| Vec::new().into())
        }
    }
}
