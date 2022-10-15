// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::js_util::referrer;
use core_protocol::id::ServerId;
use core_protocol::name::Referrer;

pub trait Frontend<P> {
    /// Set the props used to render the UI.
    fn set_ui_props(&self, props: P);
    /// Gets the referer.
    fn get_real_referrer(&self) -> Option<Referrer> {
        referrer()
    }
    /// Gets url hosting client files.
    fn get_real_host(&self) -> Option<String>;
    /// True iif should use HTTPS/WSS.
    fn get_real_encryption(&self) -> Option<bool>;
    /// Gets the server's response for ideal [`ServerId`].
    fn get_ideal_server_id(&self) -> Option<ServerId>;
}
