// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::cloud::{Cloud, DnsUpdate};
use actix::{fut, Actor, ActorFutureExt, AsyncContext, Context, ContextFutureSpawner, WrapFuture};
use actix_web::http::header::CONNECTION;
use actix_web::http::Version;
use actix_web::rt::time::sleep;
use awc::Client;
use core_protocol::id::ServerId;
use log::{error, info, warn};
use std::collections::HashSet;
use std::lazy::OnceCell;
use std::time::Duration;

/// Putting these in an actor is very tricky. They won't have a static lifetime, and that makes
/// it hard to use async functions.
static mut CLOUD: OnceCell<Box<dyn Cloud>> = OnceCell::new();

/// Monitors web servers and changes DNS to recover from servers going offline.
pub struct Watchdog {
    domain: String,
}

impl Watchdog {
    /// "" is the real home. This can be set to an alternate, inconsequential, value for testing, like "home"
    #[cfg(debug_assertions)]
    const HOME: &'static str = "home";
    #[cfg(not(debug_assertions))]
    const HOME: &'static str = "";

    /// How many tries to connect to a serve before assuming dead.
    const TRIES: usize = 2;

    /// How long to wait between tries
    const RETRY: Duration = Duration::from_secs(5);

    /// # Safety
    ///
    /// Only every call once.
    pub unsafe fn new(cloud: Box<dyn Cloud>, domain: String) -> Self {
        CLOUD
            .set(cloud)
            .unwrap_or_else(|_| panic!("unable to set cloud"));

        Self { domain }
    }

    fn cloud() -> &'static dyn Cloud {
        unsafe { CLOUD.get().unwrap().as_ref() }
    }
}

impl Actor for Watchdog {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_secs(300), |act, ctx| {
            let domain = act.domain.to_owned();

            Box::pin(async move {
                match Self::cloud().read_dns(&domain).await {
                    Ok(res) => {
                        let client = Client::builder()
                            .timeout(Duration::from_secs(5))
                            .max_http_version(Version::HTTP_11)
                            .add_default_header((CONNECTION, "close"))
                            .finish();

                        // Servers currently hosting the home page.
                        let mut home = Vec::new();

                        // Alive servers.
                        let mut alive = HashSet::new();

                        for (sub_domain, mut ip_addresses) in res {
                            if sub_domain == Self::HOME {
                                home.append(&mut ip_addresses);
                            } else if let Some(server_id) =
                                sub_domain.parse::<u8>().ok().and_then(ServerId::new)
                            {
                                if ip_addresses.len() == 1 {
                                    for i in 0..Self::TRIES {
                                        let mut response = match client
                                            .get(format!(
                                                "https://{}.{}/status/",
                                                sub_domain, domain
                                            ))
                                            .send()
                                            .await
                                        {
                                            Ok(r) => r,
                                            Err(e) => {
                                                warn!(
                                                    "could not reach server {:?}: {}",
                                                    server_id, e
                                                );
                                                if i != Self::TRIES - 1 {
                                                    sleep(Self::RETRY).await;
                                                }
                                                continue;
                                            }
                                        };

                                        let _body = match response.body().await {
                                            Ok(b) => b,
                                            Err(e) => {
                                                warn!(
                                                    "could not reach server {:?}: {}",
                                                    server_id, e
                                                );
                                                if i != Self::TRIES - 1 {
                                                    sleep(Self::RETRY).await;
                                                }
                                                continue;
                                            }
                                        };

                                        /*
                                        let body: AdminUpdate = match response.json().await {
                                            Ok(b) => b,
                                            Err(e) => {
                                                println!("could not reach server {:?}: {}", server_id, e);
                                                break;
                                            }
                                        };
                                         */

                                        alive.insert(ip_addresses.pop().unwrap());
                                        break;
                                    }
                                } else {
                                    error!(
                                        "unexpected number of ip addresses for server {:?}: {:?}",
                                        server_id, ip_addresses
                                    )
                                }
                            }
                        }

                        if alive.is_empty() {
                            error!("there are no alive servers to promote");
                        } else {
                            for dead in home.drain_filter(|server| !alive.contains(server)) {
                                warn!("removing dead server {} from home", dead);
                                if let Err(e) = Self::cloud()
                                    .update_dns(&domain, Self::HOME, DnsUpdate::Remove(dead))
                                    .await
                                {
                                    error!("error removing server {} from home: {}", dead, e);
                                }
                            }

                            if home.is_empty() {
                                if let Some(alive) = alive.into_iter().next() {
                                    warn!("promoting alive server {} to home", alive);
                                    if let Err(e) = Self::cloud()
                                        .update_dns(&domain, Self::HOME, DnsUpdate::Add(alive))
                                        .await
                                    {
                                        error!("error promoting server {} to home: {}", alive, e);
                                    }
                                }
                            } else {
                                info!(
                                    "the following alive servers are hosting the homepage: {:?}",
                                    home
                                );
                            }
                        }
                    }
                    Err(e) => error!("watchdog error: {}", e),
                }
            })
            .into_actor(act)
            .then(|_res, _act, _ctx| fut::ready(()))
            .wait(ctx);
        });
    }
}
