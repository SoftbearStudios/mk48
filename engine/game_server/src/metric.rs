// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::client::{Authenticate, PlayerClientData};
use crate::game_service::GameArenaService;
use crate::infrastructure::Infrastructure;
use crate::player::PlayerData;
use crate::system::SystemRepo;
use crate::unwrap_or_return;
use actix::Context as ActorContext;
use actix::{ActorFutureExt, ContextFutureSpawner, WrapFuture};
use core_protocol::dto::{MetricFilterDto, MetricsDataPointDto};
use core_protocol::id::{RegionId, SessionId, UserAgentId};
use core_protocol::name::Referrer;
use core_protocol::{get_unix_time_now, UnixTime};
use heapless::HistoryBuffer;
use log::error;
use server_util::database_schema::{Metrics, MetricsItem, SessionItem};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

/// Stores and updates metrics to increase observability.
pub(crate) struct MetricRepo<G: GameArenaService> {
    next_update: UnixTime,
    next_swap: UnixTime,
    current: MetricBundle,
    pub history: HistoryBuffer<MetricBundle, 24>,
    _spooky: PhantomData<G>,
}

/// Metric related data stored per client.
#[derive(Debug)]
pub struct ClientMetricData<G: GameArenaService> {
    /// Summary of domain that referred client.
    pub referrer: Option<Referrer>,
    /// General geographic location of the client.
    pub region_id: Option<RegionId>,
    /// Client user agent high level id.
    pub user_agent_id: Option<UserAgentId>,
    /// Frames per second.
    pub fps: Option<f32>,
    /// Milliseconds of network a.k.a. latency round trip time.
    pub rtt: Option<u16>,
    /// When this session was created, for database purposes.
    pub date_created: UnixTime,
    /// When this session was renewed (new websocket, etc.), for database purposes.
    pub date_renewed: UnixTime,
    /// When initial session was created, for database purposes.
    pub date_previous: Option<UnixTime>,
    /// Earlier session id (TODO: Include arena id too).
    pub session_id_previous: Option<SessionId>,
    /// When this session was created, for metrics purposes.
    pub created: Instant,
    /// When the current play was started, for metrics purposes.
    pub play_started: Option<Instant>,
    /// When the last play was stopped, for metrics purposes.
    pub play_stopped: Option<Instant>,
    /// When the current visit was started.
    pub visit_started: Option<Instant>,
    /// When the current visit was stopped.
    pub visit_stopped: Option<Instant>,
    /// How many plays on this session, for database purposes.
    pub plays: u32,
    /// How many plays on the current visit.
    pub visit_plays: u32,
    /// How many plays on previous sessions, for database purposes.
    pub previous_plays: u32,
    _spooky: PhantomData<G>,
}

/// Initializes from authenticate. Sets database fields to default values.
impl<G: GameArenaService> From<&Authenticate> for ClientMetricData<G> {
    fn from(auth: &Authenticate) -> Self {
        Self {
            user_agent_id: auth.user_agent_id,
            referrer: auth.referrer,
            region_id: auth.ip_address.and_then(SystemRepo::<G>::ip_to_region_id),
            fps: None,
            rtt: None,
            date_created: get_unix_time_now(),
            date_renewed: get_unix_time_now(),
            created: Instant::now(),
            date_previous: None,
            session_id_previous: None,
            play_started: None,
            play_stopped: None,
            visit_started: None,
            visit_stopped: None,
            plays: 0,
            visit_plays: 0,
            previous_plays: 0,
            _spooky: PhantomData,
        }
    }
}

impl<G: GameArenaService> ClientMetricData<G> {
    pub(crate) fn supplement(&mut self, session_item: &SessionItem) {
        self.session_id_previous = Some(session_item.session_id);
        self.date_previous = Some(
            session_item
                .date_previous
                .unwrap_or(session_item.date_created),
        );
        self.previous_plays = session_item.plays;
    }
}

/// Stores a T for each of several queries, and an aggregate.
#[derive(Default)]
struct Bundle<T> {
    total: T,
    by_referrer: HashMap<Referrer, T>,
    by_region_id: HashMap<RegionId, T>,
    by_user_agent_id: HashMap<UserAgentId, T>,
}

impl<T: Default> Bundle<T> {
    pub fn mutate(
        &mut self,
        mut mutation: impl FnMut(&mut T),
        referrer: Option<Referrer>,
        region_id: Option<RegionId>,
        user_agent_id: Option<UserAgentId>,
    ) {
        mutation(&mut self.total);
        if let Some(referrer) = referrer {
            // We cap at the first few referrers we see to avoid unbounded memory.
            let referrers_full = self.by_referrer.len() >= 128;

            match self.by_referrer.entry(referrer) {
                Entry::Occupied(occupied) => mutation(occupied.into_mut()),
                Entry::Vacant(vacant) => {
                    if !referrers_full {
                        mutation(vacant.insert(T::default()))
                    }
                }
            }
        }
        if let Some(region_id) = region_id {
            mutation(self.by_region_id.entry(region_id).or_default());
        }
        if let Some(user_agent_id) = user_agent_id {
            mutation(self.by_user_agent_id.entry(user_agent_id).or_default())
        }
    }

    pub fn mutate_all(&mut self, mut mutation: impl FnMut(&mut T)) {
        mutation(&mut self.total);
        for m in self.by_referrer.values_mut() {
            mutation(m);
        }
        for m in self.by_region_id.values_mut() {
            mutation(m);
        }
        for m in self.by_user_agent_id.values_mut() {
            mutation(m);
        }
    }

    /// Applies another bundle to this one, component-wise.
    pub fn apply<O>(&mut self, other: Bundle<O>, mut map: impl FnMut(&mut T, O)) {
        map(&mut self.total, other.total);
        for (referrer, o) in other.by_referrer {
            map(self.by_referrer.entry(referrer).or_default(), o);
        }
        for (region_id, o) in other.by_region_id {
            map(self.by_region_id.entry(region_id).or_default(), o);
        }
        for (user_agent_id, o) in other.by_user_agent_id {
            map(self.by_user_agent_id.entry(user_agent_id).or_default(), o);
        }
    }

    pub fn get(&self, filter: Option<MetricFilterDto>) -> Option<&T> {
        match filter {
            None => Some(&self.total),
            Some(MetricFilterDto::Referrer(referrer)) => self.by_referrer.get(&referrer),
            Some(MetricFilterDto::RegionId(region_id)) => self.by_region_id.get(&region_id),
            Some(MetricFilterDto::UserAgentId(user_agent_id)) => {
                self.by_user_agent_id.get(&user_agent_id)
            }
        }
    }
}

/// Metrics total, and by various key types.
pub(crate) struct MetricBundle {
    pub(crate) start: UnixTime,
    bundle: Bundle<Metrics>,
}

impl MetricBundle {
    pub fn new(start: UnixTime) -> Self {
        Self {
            start,
            bundle: Bundle::default(),
        }
    }

    pub fn metric(&self, filter: Option<MetricFilterDto>) -> Metrics {
        self.bundle
            .get(filter)
            .cloned()
            .unwrap_or_else(|| Metrics::default())
    }

    pub fn data_point(&self, filter: Option<MetricFilterDto>) -> MetricsDataPointDto {
        self.bundle
            .get(filter)
            .map(|m| m.data_point())
            .unwrap_or_else(|| Metrics::default().data_point())
    }
}

impl<G: GameArenaService> MetricRepo<G> {
    /// Speed up time by 60X to help debug.
    #[cfg(debug_assertions)]
    const MINUTE_IN_MILLIS: u64 = 1000;
    #[cfg(not(debug_assertions))]
    const MINUTE_IN_MILLIS: u64 = 60 * 1000;
    const HOUR_IN_MILLIS: u64 = 60 * Self::MINUTE_IN_MILLIS;
    const DAY_IN_MILLIS: u64 = 24 * Self::HOUR_IN_MILLIS;
    const MIN_VISIT_GAP: Duration = Duration::from_secs(30 * 60);

    pub fn new() -> Self {
        let now = get_unix_time_now();
        let current = MetricBundle::new(Self::round_down_to_hour(now));
        Self {
            next_swap: current.start + Self::HOUR_IN_MILLIS,
            next_update: Self::round_down_to_minute(now) + Self::MINUTE_IN_MILLIS,
            current,
            history: HistoryBuffer::default(),
            _spooky: PhantomData,
        }
    }

    pub fn mutate_all(&mut self, mutation: impl FnMut(&mut Metrics)) {
        self.current.bundle.mutate_all(mutation);
    }

    pub fn mutate_with(
        &mut self,
        mutation: impl Fn(&mut Metrics),
        client_metric_data: &ClientMetricData<G>,
    ) {
        self.current.bundle.mutate(
            mutation,
            client_metric_data.referrer,
            client_metric_data.region_id,
            client_metric_data.user_agent_id,
        );
    }

    /// Call when a websocket connects.
    pub fn start_visit(&mut self, client: &mut PlayerClientData<G>) {
        let renewed = client.metrics.session_id_previous.is_some()
            || client.metrics.previous_plays > 0
            || client.metrics.visit_stopped.is_some();

        debug_assert!(
            client.metrics.visit_started.is_none(),
            "visit already started"
        );
        client.metrics.visit_stopped = None;
        client.metrics.visit_started = Some(Instant::now());

        self.mutate_with(
            |m| {
                m.visits.increment();
                m.invited
                    .push(client.invitation.invitation_accepted.is_some());
                if renewed {
                    m.renews.increment();
                }
                // Here, we trust the client to send valid data. If it sent invalid an invalid
                // id, we will under-count new. However, we can't really stop the client from
                // forcing us to over-count new (by not sending a session despite having it).
                m.new.push(!renewed);
                m.no_referrer.push(client.metrics.referrer.is_none());
            },
            &client.metrics,
        );
    }

    pub fn start_play(&mut self, player: &mut PlayerData<G>) {
        let client = unwrap_or_return!(player.client_mut());

        debug_assert!(client.metrics.play_started.is_none(), "already started");

        let now = Instant::now();

        if let Some(date_play_stopped) = client.metrics.play_stopped {
            let elapsed = now - date_play_stopped;

            if elapsed > Self::MIN_VISIT_GAP {
                self.mutate_with(|m| m.visits.increment(), &client.metrics);
            }

            client.metrics.play_stopped = None;
        }

        client.metrics.play_started = Some(now);
        client.metrics.plays += 1;
        client.metrics.visit_plays += 1;
        self.mutate_with(|m| m.plays_total.increment(), &client.metrics)
    }

    pub fn stop_play(&mut self, player: &mut PlayerData<G>) {
        let teamed = player.team_id().is_some();
        let client = unwrap_or_return!(player.client_mut());

        debug_assert!(client.metrics.play_stopped.is_none(), "already stopped");

        let now = Instant::now();

        if let Some(play_started) = client.metrics.play_started {
            let elapsed = now - play_started;

            self.mutate_with(
                |m| {
                    m.minutes_per_play
                        .push(elapsed.as_secs_f32() * (1.0 / 60.0));
                    m.teamed.push(teamed);
                },
                &client.metrics,
            );

            client.metrics.play_started = None;
        } else {
            debug_assert!(false, "wasn't started");
        }

        client.metrics.play_stopped = Some(now);
    }

    pub fn stop_visit(&mut self, player: &mut PlayerData<G>) {
        let mut client = unwrap_or_return!(player.client_mut());

        if client.metrics.play_started.is_some() {
            debug_assert!(
                false,
                "technically valid, but play should have been stopped long ago"
            );
            self.stop_play(player);
            // Re-borrow.
            client = unwrap_or_return!(player.client_mut());
        }

        let now = Instant::now();

        let session_end = client
            .metrics
            .play_stopped
            .unwrap_or(client.metrics.created);
        let session_duration = session_end - client.metrics.created;

        debug_assert!(client.metrics.visit_started.is_some());
        let minutes_per_visit = client
            .metrics
            .visit_started
            .map(|visit_started| (now - visit_started).as_secs_f32() * (1.0 / 60.0));

        self.mutate_with(
            |m| {
                m.bounce.push(client.metrics.plays == 0);
                if client.metrics.plays > 0 {
                    let peek_flop =
                        client.metrics.plays == 1 && session_duration < Duration::from_secs(60);
                    if client.metrics.date_previous.is_some() {
                        // Returning player left promptly.
                        m.peek.push(peek_flop);
                    } else {
                        // New player left promptly.
                        m.flop.push(peek_flop);
                    }
                    if let Some(minutes_per_visit) = minutes_per_visit {
                        m.minutes_per_visit.push(minutes_per_visit);
                    }
                    m.plays_per_visit.push(client.metrics.visit_plays as f32);
                }
            },
            &client.metrics,
        );

        client.metrics.visit_started = None;
        client.metrics.visit_stopped = Some(Instant::now());
        client.metrics.visit_plays = 0;
    }

    /// Returns metric to safe in database, if any.
    pub fn update(infrastructure: &mut Infrastructure<G>) -> Option<MetricsItem> {
        let metrics_repo = &mut infrastructure.metrics;

        let now = get_unix_time_now();

        if now < metrics_repo.next_update {
            return None;
        }
        metrics_repo.next_update = Self::round_down_to_minute(now) + Self::MINUTE_IN_MILLIS;

        let context = &mut infrastructure.context_service.context;
        let uptime = infrastructure.status.uptime();
        let health = &mut infrastructure.status.health;

        let mut concurrent = Bundle::<u32>::default();

        for player in context.players.iter_borrow() {
            if !player.is_alive() {
                continue;
            }
            if let Some(client) = player.client() {
                concurrent.mutate(
                    |c| *c += 1,
                    client.metrics.referrer,
                    client.metrics.region_id,
                    client.metrics.user_agent_id,
                );
                metrics_repo.mutate_with(
                    |m| {
                        if let Some(fps) = client.metrics.fps {
                            m.fps.push(fps);
                            m.low_fps.push(fps < 24.0);
                        }
                        if let Some(rtt) = client.metrics.rtt {
                            m.rtt.push(rtt as f32 * 0.001);
                        }
                        m.score.push(player.score as f32);

                        let retention_millis = now.saturating_sub(
                            client
                                .metrics
                                .date_previous
                                .unwrap_or(client.metrics.date_created),
                        );
                        let retention =
                            (retention_millis as f64 * (1.0 / Self::DAY_IN_MILLIS as f64)) as f32;
                        m.retention_days.push(retention);
                        m.retention_histogram.push(retention);
                    },
                    &client.metrics,
                );
            }
        }

        metrics_repo
            .current
            .bundle
            .apply(concurrent, |metrics, concurrent| {
                if concurrent > 0 {
                    metrics.concurrent.push(concurrent as f32)
                }
            });

        metrics_repo.mutate_all(|m| {
            m.cpu.push(health.cpu());
            m.cpu_steal.push(health.cpu_steal());
            m.ram.push(health.ram());
            const MEGABIT: f32 = 125000.0;
            m.bandwidth_rx.push(health.bandwidth_rx() as f32 / MEGABIT);
            m.bandwidth_tx.push(health.bandwidth_tx() as f32 / MEGABIT);
            m.connections.push(health.connections() as f32);
            m.tps = m.tps + health.take_tps();
            m.spt = m.spt + health.take_spt();
            m.uptime.push(uptime.as_secs_f32() / (24.0 * 60.0 * 60.0));
        });

        if now < metrics_repo.next_swap {
            return None;
        }
        let new_current = Self::round_down_to_hour(now);
        metrics_repo.next_swap = new_current + Self::HOUR_IN_MILLIS;

        let mut current = MetricBundle::new(metrics_repo.current.start);
        current.bundle.total = Self::get_metrics(infrastructure, None);
        for user_agent_id in infrastructure
            .metrics
            .current
            .bundle
            .by_user_agent_id
            .keys()
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
        {
            current.bundle.by_user_agent_id.insert(
                user_agent_id,
                Self::get_metrics(
                    infrastructure,
                    Some(MetricFilterDto::UserAgentId(user_agent_id)),
                ),
            );
        }
        for referrer in infrastructure
            .metrics
            .current
            .bundle
            .by_referrer
            .keys()
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
        {
            current.bundle.by_referrer.insert(
                referrer,
                Self::get_metrics(infrastructure, Some(MetricFilterDto::Referrer(referrer))),
            );
        }
        for region_id in infrastructure
            .metrics
            .current
            .bundle
            .by_region_id
            .keys()
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
        {
            current.bundle.by_region_id.insert(
                region_id,
                Self::get_metrics(infrastructure, Some(MetricFilterDto::RegionId(region_id))),
            );
        }

        let save_to_db = MetricsItem {
            game_id: G::GAME_ID,
            timestamp: current.start,
            metrics: current.bundle.total.clone(),
        };

        infrastructure.metrics.history.write(current);
        infrastructure.metrics.current = MetricBundle::new(new_current);

        Some(save_to_db)
    }

    pub fn update_to_database(
        infrastructure: &mut Infrastructure<G>,
        ctx: &mut ActorContext<Infrastructure<G>>,
    ) {
        if let Some(metrics_item) = Self::update(infrastructure) {
            let server_number = infrastructure.server_id.map(|id| id.0.get()).unwrap_or(0);
            let database = infrastructure.database();

            async move {
                // Don't hammer the database row from multiple servers simultaneously, which
                // wouldn't compromise correctness, but would affect performance (number of retries).
                tokio::time::sleep(Duration::from_secs(server_number as u64 * 5 + 1)).await;
                database.update_metrics(metrics_item).await
            }
            .into_actor(infrastructure)
            .map(|res, _, _| {
                if let Err(e) = res {
                    error!("error putting metrics: {:?}", e)
                }
            })
            .spawn(ctx)
        }
    }

    pub fn get_metrics(
        infrastructure: &mut Infrastructure<G>,
        filter: Option<MetricFilterDto>,
    ) -> Metrics {
        // Get basis.
        let metrics_repo = &mut infrastructure.metrics;
        let mut metrics = metrics_repo
            .current
            .bundle
            .get(filter)
            .cloned()
            .unwrap_or_default();

        // For now, the infrastructure is always hosting one arena.
        metrics.arenas_cached.increment();
        metrics
            .players_cached
            .add_length(infrastructure.context_service.context.players.len());
        metrics
            .sessions_cached
            .add_length(infrastructure.context_service.context.players.real_players);
        metrics
            .invitations_cached
            .add_length(infrastructure.invitations.len());

        metrics
    }

    /// Rounds down the time to the nearest minute.
    fn round_down_to_minute(time: UnixTime) -> UnixTime {
        (time / Self::MINUTE_IN_MILLIS) * Self::MINUTE_IN_MILLIS
    }

    /// Rounds down the time to the nearest hour.
    fn round_down_to_hour(time: UnixTime) -> UnixTime {
        (time / Self::HOUR_IN_MILLIS) * Self::HOUR_IN_MILLIS
    }
}
