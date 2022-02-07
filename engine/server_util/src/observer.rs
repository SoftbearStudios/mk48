// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix::prelude::*;
use actix::Recipient;

#[derive(Message)]
#[rtype(result = "()")]
pub enum ObserverMessage<I, O, P = ()>
where
    O: actix::Message + std::marker::Send,
    P: Clone,
    <O as actix::Message>::Result: std::marker::Send,
{
    Request {
        observer: Recipient<ObserverUpdate<O>>,
        request: I,
    },
    RoundTripTime {
        observer: Recipient<ObserverUpdate<O>>,
        /// Unique measurement of the round trip time, in milliseconds.
        rtt: u16,
    },
    Register {
        observer: Recipient<ObserverUpdate<O>>,
        payload: P,
    },
    Unregister {
        observer: Recipient<ObserverUpdate<O>>,
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
