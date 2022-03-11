// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix::prelude::*;
use core_protocol::id::PlayerId;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Message)]
#[rtype(result = "()")]
pub enum ObserverMessage<I, O>
where
    O: actix::Message + std::marker::Send,
    <O as actix::Message>::Result: std::marker::Send,
{
    Request {
        player_id: PlayerId,
        request: I,
    },
    RoundTripTime {
        player_id: PlayerId,
        /// Unique measurement of the round trip time, in milliseconds.
        rtt: u16,
    },
    Register {
        player_id: PlayerId,
        observer: UnboundedSender<ObserverUpdate<O>>,
    },
    Unregister {
        player_id: PlayerId,
        observer: UnboundedSender<ObserverUpdate<O>>,
    },
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub enum ObserverUpdate<O>
where
    O: actix::Message + std::marker::Send,
    <O as actix::Message>::Result: std::marker::Send,
{
    Close,
    Send { message: O },
}
