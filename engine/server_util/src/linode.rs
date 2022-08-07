// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::cloud::{Cloud, DnsUpdate};
use async_trait::async_trait;
use log::error;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration;

pub struct Linode {
    client: Client,
}

impl Linode {
    const ERR_TEXT: &'static str = "error with Linode API";
    const TTL: usize = 30;

    pub fn new(personal_access_token: &str) -> Self {
        let mut default_headers = HeaderMap::new();

        default_headers.insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", personal_access_token)).unwrap(),
        );
        default_headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_str("softbear game server").unwrap(),
        );

        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(5))
                .default_headers(default_headers)
                .http1_only()
                .build()
                .unwrap(),
        }
    }

    fn map_error(e: reqwest::Error) -> &'static str {
        error!("{}", e);
        Self::ERR_TEXT
    }

    async fn list_domains(&self) -> Result<LinodeListDomainsResponse, &'static str> {
        let endpoint = "https://api.linode.com/v4/domains";
        let request = self.client.get(endpoint);
        let response = request.send().await.map_err(Self::map_error)?;
        response.json().await.map_err(Self::map_error)
    }

    async fn get_domain_id(&self, domain: &str) -> Result<usize, &'static str> {
        let list: LinodeListDomainsResponse = self.list_domains().await?;
        list.data
            .iter()
            .find(|d| d.domain == domain)
            .map(|d| d.id)
            .ok_or("Could not find domain")
    }

    async fn list_domains_records(
        &self,
        domain_id: usize,
    ) -> Result<LinodeListDomainRecordsResponse, &'static str> {
        let endpoint = format!("https://api.linode.com/v4/domains/{}/records", domain_id);
        let request = self.client.get(endpoint);
        let response = request.send().await.map_err(Self::map_error)?;
        response.json().await.map_err(Self::map_error)
    }

    async fn create_domain_record(
        &self,
        domain_id: usize,
        record: LinodeDomainRecord,
    ) -> Result<LinodeDomainRecordResponse, &'static str> {
        let endpoint = format!("https://api.linode.com/v4/domains/{}/records", domain_id);
        let request = self.client.post(endpoint);
        let request = request.json(&record).build().map_err(Self::map_error)?;
        let response = self
            .client
            .execute(request)
            .await
            .map_err(Self::map_error)?;
        response.json().await.map_err(Self::map_error)
    }

    async fn update_domain_record(
        &self,
        domain_id: usize,
        id: usize,
        record: LinodeDomainRecord,
    ) -> Result<LinodeDomainRecordResponse, &'static str> {
        let endpoint = format!(
            "https://api.linode.com/v4/domains/{}/records/{}",
            domain_id, id
        );
        let request = self
            .client
            .put(endpoint)
            .json(&record)
            .build()
            .map_err(Self::map_error)?;
        let response = self
            .client
            .execute(request)
            .await
            .map_err(Self::map_error)?;
        response.json().await.map_err(Self::map_error)
    }

    async fn delete_domain_record(&self, domain_id: usize, id: usize) -> Result<(), &'static str> {
        let endpoint = format!(
            "https://api.linode.com/v4/domains/{}/records/{}",
            domain_id, id
        );
        let request = self
            .client
            .delete(endpoint)
            .build()
            .map_err(Self::map_error)?;
        self.client
            .execute(request)
            .await
            .map_err(Self::map_error)?;
        Ok(())
    }
}

#[async_trait(?Send)]
impl Cloud for Linode {
    async fn read_dns(&self, domain: &str) -> Result<HashMap<String, Vec<IpAddr>>, &'static str> {
        let domain_id = self.get_domain_id(domain).await?;

        let list: LinodeListDomainRecordsResponse = self.list_domains_records(domain_id).await?;

        // May be more capacity than required, but always enough.
        let mut ret = HashMap::with_capacity(list.data.len());

        for record in list
            .data
            .into_iter()
            .filter(|record| record.record.typ == LinodeDomainRecordType::A)
        {
            let ip = IpAddr::from_str(&record.record.target)
                .map_err(|_| "could not parse ip of A record")?;
            ret.entry(record.record.name)
                .or_insert_with(|| Vec::with_capacity(1))
                .push(ip);
        }

        Ok(ret)
    }

    async fn update_dns(
        &self,
        domain: &str,
        sub_domain: &str,
        update: DnsUpdate,
    ) -> Result<(), &'static str> {
        let domain_id = self.get_domain_id(domain).await?;

        let mut list: LinodeListDomainRecordsResponse =
            self.list_domains_records(domain_id).await?;

        list.data
            .retain(|record| record.record.typ == LinodeDomainRecordType::A);

        let mut old = list.data.iter().filter(|r| r.record.name == sub_domain);

        match update {
            DnsUpdate::Set(ip) => {
                let mut new = Some(|| LinodeDomainRecord {
                    name: sub_domain.to_owned(),
                    target: ip.to_string(),
                    ttl_sec: Self::TTL,
                    typ: LinodeDomainRecordType::A,
                });

                for record in old {
                    let record_ip = IpAddr::from_str(&record.record.target)
                        .map_err(|_| "could not parse IP address of A record")?;
                    if record_ip == ip {
                        new = None;
                    } else if let Some(new) = new.take() {
                        self.update_domain_record(domain_id, record.id, new())
                            .await?;
                    } else {
                        self.delete_domain_record(domain_id, record.id).await?;
                    }
                }

                if let Some(new) = new {
                    self.create_domain_record(domain_id, new()).await?;
                }
            }
            DnsUpdate::Add(ip) => {
                if !old.any(|r| {
                    IpAddr::from_str(&r.record.target)
                        .map(|record_ip| record_ip == ip)
                        .unwrap_or(false)
                }) {
                    let new = LinodeDomainRecord {
                        name: sub_domain.to_owned(),
                        target: ip.to_string(),
                        ttl_sec: Self::TTL,
                        typ: LinodeDomainRecordType::A,
                    };

                    self.create_domain_record(domain_id, new).await?;
                }
            }
            DnsUpdate::Remove(ip) => {
                for record in old {
                    if IpAddr::from_str(&record.record.target)
                        .map_err(|_| "could not parse A record ip")?
                        == ip
                    {
                        self.delete_domain_record(domain_id, record.id).await?;
                    }
                }
            }
            DnsUpdate::Clear => {
                for record in old {
                    self.delete_domain_record(domain_id, record.id).await?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct LinodeListDomainsResponse {
    data: Vec<LinodeDomainResponse>,
}

#[derive(Debug, Deserialize)]
struct LinodeDomainResponse {
    id: usize,
    domain: String,
}

#[derive(Debug, Deserialize)]
struct LinodeListDomainRecordsResponse {
    data: Vec<LinodeDomainRecordResponse>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
struct LinodeDomainRecord {
    name: String,
    target: String,
    ttl_sec: usize,
    #[serde(rename = "type")]
    typ: LinodeDomainRecordType,
}

#[derive(Debug, Deserialize)]
struct LinodeDomainRecordResponse {
    id: usize,
    #[serde(flatten)]
    record: LinodeDomainRecord,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum LinodeDomainRecordType {
    A,
    Aaaa,
    Ns,
    Mx,
    Cname,
    Txt,
    Srv,
    Caa,
    Ptr,
}
