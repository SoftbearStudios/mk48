use crate::repo::Repo;
use crate::session::Session;
use core_protocol::dto::MetricsDto;
use core_protocol::id::GameId;
use core_protocol::metrics::*;
use core_protocol::name::Referer;
use core_protocol::{get_unix_time_now, UnixTime};
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::mem;
use std::ops::Add;
use std::sync::Arc;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Metrics {
    /// Ratio of new players that leave fast.
    pub bounce: RatioMetric,
    /// Concurrent players.
    pub concurrent: ContinuousExtremaMetric,
    /// Minutes per session.
    pub minutes: ContinuousExtremaMetric,
    /// Ratio of unique players that are new to players that are not.
    pub new: RatioMetric,
    /// Ratio of regular players that leave fast.
    pub peek: RatioMetric,
    /// Minutes per play.
    pub play_minutes: ContinuousExtremaMetric,
    /// Plays per session.
    pub plays: ContinuousExtremaMetric,
    /// Player retention in days.
    pub retention: ContinuousExtremaMetric,
    /// Score per play.
    pub score: ContinuousExtremaMetric,
    /// Ratio of plays that end team-less to plays that don't.
    pub solo: RatioMetric,
}

impl Metrics {
    pub fn summarize(&self) -> MetricsDto {
        MetricsDto {
            bounce: self.bounce.summarize(),
            concurrent: self.concurrent.summarize(),
            minutes: self.minutes.summarize(),
            new: self.new.summarize(),
            peek: self.peek.summarize(),
            play_minutes: self.play_minutes.summarize(),
            plays: self.plays.summarize(),
            retention: self.retention.summarize(),
            score: self.score.summarize(),
            solo: self.solo.summarize(),
        }
    }
}

// TODO: Consider deriving this with derive_more crate.
impl Add for Metrics {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            bounce: self.bounce + rhs.bounce,
            concurrent: self.concurrent + rhs.concurrent,
            minutes: self.minutes + rhs.minutes,
            new: self.new + rhs.new,
            peek: self.peek + rhs.peek,
            play_minutes: self.play_minutes + rhs.play_minutes,
            plays: self.plays + rhs.plays,
            retention: self.retention + rhs.retention,
            score: self.score + rhs.score,
            solo: self.solo + rhs.solo,
        }
    }
}

impl Repo {
    // Returns the refers so that the caller can create a filter for `get_metrics()`.
    pub fn _get_referers(&mut self) -> Arc<[Referer]> {
        debug!("get_referers()");

        let mut hash: HashSet<Referer> = HashSet::new();
        for (_, arena) in self.arenas.iter() {
            for (_, session) in arena.sessions.iter() {
                if let Some(referer) = session.referer {
                    hash.insert(referer.clone());
                }
            }
        }
        mem::take(&mut hash).into_iter().collect()
    }

    // Returns the metrics for the most recent 24-hour period.
    pub fn get_metrics<F>(&mut self, period_millis: u64, filter: F) -> HashMap<GameId, Metrics>
    where
        F: Fn(&Session) -> bool,
    {
        debug!("get_metrics()");

        let mut ret: HashMap<GameId, Metrics> = HashMap::new();

        // Warning: failure to use this on both operands of a subtraction may induce an overflow.
        fn floor_to_minute(time: UnixTime) -> UnixTime {
            (time / 60000) * 60000
        }

        let now = get_unix_time_now();
        let metric_stop = floor_to_minute(now);
        let metric_start = metric_stop - period_millis;

        for (_, arena) in self.arenas.iter() {
            let metrics = ret.entry(arena.game_id).or_default();
            let mut minute_buckets = vec![0u32; (period_millis / 1000) as usize]; // Number of players in each 1 minute bucket over 24 hours.
            let mut unique_visitors = HashSet::new();

            for (_, session) in arena.sessions.iter() {
                if session.bot {
                    continue;
                }
                let session_stop = session.date_terminated.unwrap_or(metric_stop);
                if session_stop < metric_start {
                    continue;
                }
                if !filter(session) {
                    continue;
                }

                let session_start = floor_to_minute(session.date_created);
                let days = (session_stop - session_start) as f32 / (24 * 60 * 60 * 1000) as f32;
                metrics.retention.push(days);

                let mut bounced = true;
                let mut play_count = 0;
                let mut total_minutes = 0;
                // Next bucket to insert into (to avoid duplicating).
                let mut next_bucket = 0;
                for play in session.plays.iter() {
                    let play_stop = play.date_stop.unwrap_or(metric_stop);

                    if play_stop < metric_start {
                        // Exclude plays prior to start (24h ago).
                        continue;
                    }

                    let play_start = metric_start.max(floor_to_minute(play.date_created));
                    let minutes = (play_stop - play_start) / 60000;
                    if minutes != 0 || session.plays.len() > 1 {
                        bounced = false;
                        play_count += 1;
                        metrics.minutes.push(minutes as f32);
                        total_minutes += minutes;
                        if let Some(score) = play.score {
                            metrics.score.push(score as f32);
                        }

                        metrics.solo.push(play.team_id.is_none());

                        let minute_start: u32 =
                            (play_start.saturating_sub(metric_start) / 60000) as u32;
                        let minute_stop: u32 =
                            (play_stop.saturating_sub(metric_start) / 60000) as u32;
                        for m in minute_start.max(next_bucket)
                            ..minute_stop.min(minute_buckets.len() as u32)
                        {
                            minute_buckets[m as usize] += 1;
                            next_bucket = m + 1;
                        }
                    }
                } // for play

                metrics.play_minutes.push(total_minutes as f32);
                metrics.plays.push(play_count as f32);
                if unique_visitors.insert(session.player_id) {
                    metrics.new.push(session.previous_id.is_none());
                    metrics.bounce.push(bounced);
                } else {
                    metrics.peek.push(bounced);
                }
            } // for session

            for bucket in minute_buckets {
                metrics.concurrent.push(bucket as f32);
            }
        } // for arena

        ret
    }
}
