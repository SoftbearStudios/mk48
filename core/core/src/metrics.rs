// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::repo::Repo;
use crate::session::Session;
use core_protocol::dto::MetricsDto;
use core_protocol::id::{GameId, UserAgentId};
use core_protocol::metrics::*;
use core_protocol::name::Referrer;
use core_protocol::{get_unix_time_now, UnixTime};
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::ops::Add;
use std::sync::Arc;
use sysinfo::{ProcessorExt, SystemExt};

const BUCKET_MILLIS: u64 = 60000;
const MINUTE_IN_MILLIS: u64 = 60000;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Metrics {
    /// How many arenas are in cache.
    pub arenas_cached: DiscreteMetric,
    /// Ratio of new players that leave without ever playing.
    pub bounce: RatioMetric,
    /// Concurrent players.
    pub concurrent: ContinuousExtremaMetric,
    /// Percent of available server CPU required by service.
    pub cpu: ContinuousExtremaMetric,
    /// Ratio of new players that play only once and leave quickly.
    pub flop: RatioMetric,
    /// Ratio of new players who were invited to new players who were not.
    pub invited: RatioMetric,
    /// Minutes per completed play (a measure of engagagement).
    pub minutes_per_play: ContinuousExtremaMetric,
    /// Minutes played, per session, during the metrics period.
    pub minutes_per_session: ContinuousExtremaMetric,
    /// Ratio of unique players that are new to players that are not.
    pub new: RatioMetric,
    /// Ratio of previous players that leave without playing (e.g. to peek at player count).
    pub peek: RatioMetric,
    /// Plays per session (a measure of engagagement).
    pub plays_per_session: ContinuousExtremaMetric,
    /// Plays total (aka impressions).
    pub plays_total: DiscreteMetric,
    /// Percent of available server RAM required by service.
    pub ram: ContinuousExtremaMetric,
    /// Player retention in days.
    pub retention: ContinuousExtremaMetric,
    /// Score per completed play.
    pub score: ContinuousExtremaMetric,
    /// Total sessions in cache.
    pub sessions_cached: DiscreteMetric,
    /// Ratio of plays that end team-less to plays that don't.
    pub teamed: RatioMetric,
    /// Ratio of inappropriate messages to total.
    pub toxicity: RatioMetric,
    /// Uptime in (fractional) days.
    pub uptime: ContinuousExtremaMetric,
}

impl Metrics {
    pub fn summarize(&self) -> MetricsDto {
        MetricsDto {
            arenas_cached: self.arenas_cached.summarize(),
            bounce: self.bounce.summarize(),
            concurrent: self.concurrent.summarize(),
            cpu: self.cpu.summarize(),
            flop: self.flop.summarize(),
            invited: self.invited.summarize(),
            minutes_per_play: self.minutes_per_play.summarize(),
            minutes_per_session: self.minutes_per_session.summarize(),
            new: self.new.summarize(),
            peek: self.peek.summarize(),
            plays_per_session: self.plays_per_session.summarize(),
            plays_total: self.plays_total.summarize(),
            ram: self.ram.summarize(),
            retention: self.retention.summarize(),
            score: self.score.summarize(),
            sessions_cached: self.sessions_cached.summarize(),
            teamed: self.teamed.summarize(),
            toxicity: self.toxicity.summarize(),
            uptime: self.uptime.summarize(),
        }
    }
}

// TODO: Consider deriving this with derive_more crate.
impl Add for Metrics {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            arenas_cached: self.arenas_cached + rhs.arenas_cached,
            bounce: self.bounce + rhs.bounce,
            concurrent: self.concurrent + rhs.concurrent,
            cpu: self.cpu + rhs.cpu,
            flop: self.flop + rhs.flop,
            invited: self.invited + rhs.invited,
            minutes_per_play: self.minutes_per_play + rhs.minutes_per_play,
            minutes_per_session: self.minutes_per_session + rhs.minutes_per_session,
            new: self.new + rhs.new,
            peek: self.peek + rhs.peek,
            plays_per_session: self.plays_per_session + rhs.plays_per_session,
            plays_total: self.plays_total + rhs.plays_total,
            ram: self.ram + rhs.ram,
            retention: self.retention + rhs.retention,
            score: self.score + rhs.score,
            sessions_cached: self.sessions_cached + rhs.sessions_cached,
            teamed: self.teamed + rhs.teamed,
            toxicity: self.toxicity + rhs.toxicity,
            uptime: self.uptime + rhs.uptime,
        }
    }
}

impl Repo {
    pub fn get_day<F>(&mut self, game_id: GameId, filter: &F) -> Arc<[(UnixTime, MetricsDto)]>
    where
        F: Fn(&Session) -> bool,
    {
        let now = get_unix_time_now();
        let day_start = now - 24 * 3600 * 1000; // TODO: start at midnight!

        let mut list = Vec::new();
        for t in (day_start..now).step_by(3600 * 1000) {
            let hour_start = t;
            let hour_stop = hour_start + 3600 * 1000;
            if let Some(metrics) =
                self.get_metrics(&game_id, Some(hour_start), Some(hour_stop), filter)
            {
                list.push((t, metrics.summarize()));
            }
        }

        list.into()
    }

    // Returns the game IDs so that the caller can create a filter for `get_metrics()`.
    pub fn get_game_ids(&mut self) -> Arc<[(GameId, f32)]> {
        debug!("get_game_ids()");

        let mut hash: HashMap<GameId, u32> = HashMap::new();
        let mut total = 0;
        for (_, arena) in self.arenas.iter() {
            for (_, session) in arena.sessions.iter() {
                if session.bot {
                    continue;
                }
                total += 1;
                let count = hash.entry(session.game_id).or_insert(0);
                *count += 1;
            }
        }
        let mut list: Vec<(GameId, u32)> = hash.into_iter().collect();
        list.sort_by(|(_, a), (_, b)| b.cmp(&a));

        list.into_iter()
            .map(|(game_id, count)| (game_id, count as f32 / total as f32))
            .collect()
    }

    // Returns the referrers so that the caller can create a filter for `get_metrics()`.
    pub fn get_referrers(&mut self) -> Arc<[(Referrer, f32)]> {
        debug!("get_referrers()");

        let mut hash: HashMap<Referrer, u32> = HashMap::new();
        let mut total = 0;
        for (_, arena) in self.arenas.iter() {
            for (_, session) in arena.sessions.iter() {
                if session.bot || session.date_terminated.is_some() {
                    continue;
                }
                total += 1;
                if let Some(referrer) = session.referrer {
                    let count = hash.entry(referrer.clone()).or_insert(0);
                    *count += 1;
                }
            }
        }
        let mut list: Vec<(Referrer, u32)> = hash.into_iter().collect();
        list.sort_by(|(_, a), (_, b)| b.cmp(&a));
        list.truncate(10);

        list.into_iter()
            .map(|(referrer, count)| (referrer, count as f32 / total as f32))
            .collect()
    }

    // Returns the metrics for the most recent 24-hour period.
    pub fn get_metrics<F>(
        &mut self,
        game_id: &GameId,
        period_start: Option<UnixTime>,
        period_stop: Option<UnixTime>,
        filter: &F,
    ) -> Option<Metrics>
    where
        F: Fn(&Session) -> bool,
    {
        debug!("get_metrics()");

        // Warning: failure to use this on both operands of a subtraction may induce an overflow.
        fn floor_to_bucket(time: UnixTime) -> UnixTime {
            (time / BUCKET_MILLIS) * BUCKET_MILLIS
        }

        let now = get_unix_time_now();
        let clip_stop = period_stop.unwrap_or(now);
        let clip_start = period_start.unwrap_or(clip_stop.saturating_sub(24 * 3600 * 1000));

        if clip_start >= clip_stop {
            return None;
        }

        let first_bucket_start = floor_to_bucket(clip_start);

        // Round up to the nearest bucket.
        let bucket_count = clip_stop.saturating_sub(first_bucket_start) / BUCKET_MILLIS
            + if clip_stop % BUCKET_MILLIS == 0 { 0 } else { 1 };

        let mut metrics: Metrics = Metrics::default();

        self.system_status.refresh_cpu();
        self.system_status.refresh_memory();
        metrics.ram.push(
            self.system_status.used_memory() as f32 / self.system_status.total_memory() as f32,
        );
        metrics.cpu.push(
            self.system_status
                .processors()
                .iter()
                .map(|processor| processor.cpu_usage())
                .sum::<f32>()
                * 0.01
                / self.system_status.processors().len() as f32,
        );

        for (_, arena) in self.arenas.iter() {
            if game_id != game_id {
                continue;
            }
            metrics.arenas_cached.increment();
            metrics.uptime.push(
                now.saturating_sub(arena.date_created) as f32
                    * (1.0 / (1000.0 * 60.0 * 60.0 * 24.0)) as f32,
            );

            let mut concurrency_buckets = vec![0u32; bucket_count as usize];
            let mut unique_visitors = HashSet::new();

            for (_, session) in arena.sessions.iter() {
                if session.bot {
                    continue;
                }

                metrics.sessions_cached.increment();

                let session_stop = session.date_terminated.unwrap_or(clip_stop);
                if session_stop < clip_start {
                    continue;
                }
                if !filter(session) {
                    continue;
                }

                let days = session_stop.saturating_sub(session.date_created) as f32
                    / (24 * 60 * 60 * 1000) as f32;
                metrics.retention.push(days);

                let mut activity_minutes = 0.0;
                let mut bounced_or_peeked = true;
                let mut flopped = session.previous_id.is_none() && session.plays.is_empty();
                let mut play_count = 0;
                // Next bucket to insert into (to avoid duplicating).
                let mut next_bucket = 0;

                for play in session.plays.iter() {
                    let prorata_start = clip_start.max(play.date_created);
                    let prorata_stop = play
                        .date_stop
                        .map(|s| s.min(clip_stop))
                        .unwrap_or(clip_stop);

                    if prorata_stop < clip_start || prorata_start > clip_stop {
                        // Exclude plays prior to start or after end.
                        continue;
                    }

                    metrics.plays_total.increment();

                    // The minutes metric measures per player use within the metrics period.
                    let prorata_minutes =
                        prorata_stop.saturating_sub(prorata_start) as f32 / MINUTE_IN_MILLIS as f32;
                    activity_minutes += prorata_minutes;

                    // The concurrency metric measures peak concurrency within the metrics period.
                    let bucket_start: u32 = (floor_to_bucket(prorata_start)
                        .saturating_sub(first_bucket_start)
                        / BUCKET_MILLIS) as u32;
                    let bucket_stop: u32 =
                        (prorata_stop.saturating_sub(first_bucket_start) / BUCKET_MILLIS) as u32;
                    for b in bucket_start.max(next_bucket)
                        ..bucket_stop.min(concurrency_buckets.len() as u32)
                    {
                        concurrency_buckets[b as usize] += 1;
                        next_bucket = b + 1;
                    }

                    // Users who press "play" but quit quickly and never play again are "flops".
                    bounced_or_peeked = false;
                    if flopped && play.date_stop.is_none() {
                        flopped = false;
                    }
                    if flopped && prorata_stop.saturating_sub(play.date_created) >= MINUTE_IN_MILLIS
                    {
                        flopped = false;
                    }

                    // Flops don't count toward play_minutes, score, or solo.
                    if flopped {
                        continue;
                    }

                    if let Some(play_stop) = play.date_stop {
                        // The following metrics are defined in terms of completed plays.
                        metrics.teamed.push(play.team_id.is_some());
                        play_count += 1;
                        let play_minutes = play_stop.saturating_sub(play.date_created) as f32
                            / MINUTE_IN_MILLIS as f32;
                        metrics.minutes_per_play.push(play_minutes);
                        if let Some(score) = play.score {
                            metrics.score.push(score as f32);
                        }
                    }
                } // for play

                if play_count == 0
                    && (session.date_renewed < clip_start || session.date_renewed > clip_stop)
                {
                    // This session was not at all active during the clip period.
                    // Do not, for example, count it as a bounce because of that.
                    continue;
                }

                metrics.toxicity = metrics.toxicity + session.chat_history.toxicity;

                metrics.minutes_per_session.push(activity_minutes);
                // Assume that any plays in session.previous_plays were prior to desired time interval.
                metrics.plays_per_session.push(play_count as f32);

                if unique_visitors.insert(session.player_id) {
                    metrics.new.push(session.previous_id.is_none());
                }

                if session.previous_id.is_none() {
                    // Player is new.
                    metrics.bounce.push(bounced_or_peeked);

                    if !bounced_or_peeked {
                        metrics.flop.push(flopped);
                    }

                    metrics.invited.push(
                        session
                            .plays
                            .first()
                            .map(|play| play.invited)
                            .unwrap_or(false),
                    );
                } else {
                    metrics.peek.push(bounced_or_peeked);
                }
            } // for session

            for b in concurrency_buckets {
                metrics.concurrent.push(b as f32);
            }
        } // for arena

        Some(metrics)
    }

    // Returns the user agent IDs so that the caller can create a filter for `get_metrics()`.
    pub fn get_user_agent_ids(&mut self) -> Arc<[(UserAgentId, f32)]> {
        debug!("get_user_agent_ids()");

        let mut hash: HashMap<UserAgentId, u32> = HashMap::new();
        let mut total = 0;
        for (_, arena) in self.arenas.iter() {
            for (_, session) in arena.sessions.iter() {
                if session.bot || session.date_terminated.is_some() {
                    continue;
                }
                total += 1;
                if let Some(user_agent_id) = session.user_agent_id {
                    let count = hash.entry(user_agent_id).or_insert(0);
                    *count += 1;
                }
            }
        }
        let mut list: Vec<(UserAgentId, u32)> = hash.into_iter().collect();
        list.sort_by(|(_, a), (_, b)| b.cmp(&a));

        list.into_iter()
            .map(|(user_agent_id, count)| (user_agent_id, count as f32 / total as f32))
            .collect()
    }
}
