// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use async_trait::async_trait;
use std::collections::HashMap;
use std::net::IpAddr;

#[async_trait(?Send)]
pub trait Cloud {
    /// Reads mapping of DNS A records.
    async fn read_dns(&self, domain: &str) -> Result<HashMap<String, Vec<IpAddr>>, &'static str>;

    /// Performs a DNS A record change.
    async fn update_dns(
        &self,
        domain: &str,
        sub_domain: &str,
        update: DnsUpdate,
    ) -> Result<(), &'static str>;
}

pub enum DnsUpdate {
    Set(IpAddr),
    Add(IpAddr),
    Remove(IpAddr),
    Clear,
}
