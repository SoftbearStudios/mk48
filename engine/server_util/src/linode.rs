// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::cloud::{Cloud, DnsUpdate};
use actix_web::http::Version;
use async_trait::async_trait;
use awc::error::{JsonPayloadError, PayloadError, SendRequestError};
use awc::Client;
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;

pub struct Linode {
    client: Client,
}

impl Linode {
    const ERR_SEND: &'static str = "error sending request to Linode";
    const ERR_RECV: &'static str = "error receiving JSON response from Linode";
    const TTL: usize = 30;

    pub fn new(personal_access_token: &str) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(5))
                .bearer_auth(personal_access_token)
                .max_http_version(Version::HTTP_11)
                .finish(),
        }
    }

    fn map_send_error(e: SendRequestError) -> &'static str {
        error!("{}", e);
        Self::ERR_SEND
    }

    fn map_recv_error(e: PayloadError) -> &'static str {
        error!("{}", e);
        Self::ERR_RECV
    }

    fn map_json_recv_error(e: JsonPayloadError) -> &'static str {
        error!("{}", e);
        Self::ERR_RECV
    }

    async fn list_domains(&self) -> Result<LinodeListDomainsResponse, &'static str> {
        let endpoint = "https://api.linode.com/v4/domains";
        let request = self.client.get(endpoint);
        let mut response = request.send().await.map_err(Self::map_send_error)?;
        response.json().await.map_err(Self::map_json_recv_error)
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
        let mut response = request.send().await.map_err(Self::map_send_error)?;
        response.json().await.map_err(Self::map_json_recv_error)
    }

    async fn create_domain_record(
        &self,
        domain_id: usize,
        record: LinodeDomainRecord,
    ) -> Result<LinodeDomainRecordResponse, &'static str> {
        let endpoint = format!("https://api.linode.com/v4/domains/{}/records", domain_id);
        let request = self.client.post(endpoint);
        let mut response = request
            .send_json(&record)
            .await
            .map_err(Self::map_send_error)?;
        response.json().await.map_err(Self::map_json_recv_error)
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
        let request = self.client.put(endpoint);
        let mut response = request
            .send_json(&record)
            .await
            .map_err(Self::map_send_error)?;
        response.json().await.map_err(Self::map_json_recv_error)
    }

    async fn delete_domain_record(&self, domain_id: usize, id: usize) -> Result<(), &'static str> {
        let endpoint = format!(
            "https://api.linode.com/v4/domains/{}/records/{}",
            domain_id, id
        );
        let request = self.client.delete(endpoint);
        let mut response = request.send().await.map_err(Self::map_send_error)?;
        response
            .body()
            .await
            .map(|_| ())
            .map_err(Self::map_recv_error)
    }
}

#[async_trait(?Send)]
impl Cloud for Linode {
    async fn read_dns(&self, domain: &str) -> Result<HashMap<String, Vec<IpAddr>>, &'static str> {
        let domain_id = self.get_domain_id(domain).await?;

        let list: LinodeListDomainRecordsResponse = self.list_domains_records(domain_id).await?;

        // May be more capacity than required, but always enough.
        let mut ret = HashMap::with_capacity(list.data.len());

        for record in list.data {
            ret.entry(record.record.name)
                .or_insert_with(|| Vec::with_capacity(1))
                .push(record.record.target);
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

        let list: LinodeListDomainRecordsResponse = self.list_domains_records(domain_id).await?;

        let mut old = list.data.iter().filter(|r| r.record.name == sub_domain);

        match update {
            DnsUpdate::Set(ip) => {
                let mut new = Some(|| LinodeDomainRecord {
                    name: sub_domain.to_owned(),
                    target: ip,
                    ttl_sec: Self::TTL,
                    typ: LinodeDomainRecordType::A,
                });

                for record in old {
                    if record.record.target == ip {
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
                if old.find(|r| r.record.target == ip).is_none() {
                    let new = LinodeDomainRecord {
                        name: sub_domain.to_owned(),
                        target: ip,
                        ttl_sec: Self::TTL,
                        typ: LinodeDomainRecordType::A,
                    };

                    self.create_domain_record(domain_id, new).await?;
                }
            }
            DnsUpdate::Remove(ip) => {
                for record in old {
                    if record.record.target == ip {
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
    target: IpAddr,
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
