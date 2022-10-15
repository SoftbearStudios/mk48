// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::http::{HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Redirect};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::time::Duration;

pub struct DiscordBotRepo {
    guild_id: NonZeroU64,
    client: reqwest::Client,
}

impl DiscordBotRepo {
    pub fn new(guild_id: NonZeroU64, token: &str) -> Option<Self> {
        HeaderValue::from_str(&format!("Bot {}", token))
            .ok()
            .map(|auth_header| {
                let mut default_headers = HeaderMap::new();

                default_headers.insert(reqwest::header::AUTHORIZATION, auth_header);

                Self {
                    guild_id,
                    client: reqwest::Client::builder()
                        .timeout(Duration::from_secs(3))
                        .default_headers(default_headers)
                        .build()
                        .unwrap(),
                }
            })
    }

    pub async fn send_message(
        &self,
        channel_name: &str,
        message: &str,
        reply_to_id: Option<NonZeroU64>,
    ) -> Result<(), String> {
        #[derive(Deserialize)]
        struct Channel {
            id: String,
            name: String,
        }

        let channels: Vec<Channel> = self
            .client
            .get(format!(
                "https://discord.com/api/guilds/{}/channels",
                self.guild_id
            ))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<Vec<Channel>>()
            .await
            .map_err(|e| e.to_string())?;

        let channel_id = channels
            .into_iter()
            .find(|c| c.name == channel_name)
            .map(|c| c.id)
            .ok_or_else(|| String::from("could not find channel"))?;

        #[derive(Serialize)]
        struct MessageReference {
            message_id: String,
        }

        #[derive(Serialize)]
        struct CreateMessage<'a> {
            content: &'a str,
            message_reference: Option<MessageReference>,
        }

        let create_message = CreateMessage {
            content: message,
            message_reference: reply_to_id.map(|id| MessageReference {
                message_id: id.to_string(),
            }),
        };

        self.client
            .post(format!(
                "https://discord.com/api/channels/{}/messages",
                channel_id
            ))
            .json(&create_message)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        self.client
            .post(format!(
                "https://discord.com/api/channels/{}/messages",
                channel_id
            ))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn is_moderator(&self, id: NonZeroU64) -> Result<bool, String> {
        // https://discord.com/developers/docs/resources/guild#guild-member-object
        #[derive(Debug, Deserialize)]
        struct Membership {
            roles: Vec<String>,
        }

        let membership: Membership = self
            .client
            .get(format!(
                "https://discord.com/api/guilds/{}/members/{}",
                self.guild_id, id
            ))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<Membership>()
            .await
            .map_err(|e| e.to_string())?;

        //println!("{:?}", membership);

        #[derive(Debug, Deserialize)]
        struct Role {
            id: String,
            name: String,
        }

        let roles: Vec<Role> = self
            .client
            .get(format!(
                "https://discord.com/api/guilds/{}/roles",
                self.guild_id
            ))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<Vec<Role>>()
            .await
            .map_err(|e| e.to_string())?;

        let roles_hash: HashMap<String, String> =
            roles.into_iter().map(|role| (role.id, role.name)).collect();

        Ok(membership.roles.iter().any(|id| {
            roles_hash
                .get(&*id)
                .map(|name| matches!(name.as_str(), "Developer" | "Moderator"))
                .unwrap_or(false)
        }))
    }
}

pub struct DiscordOauth2Repo {
    oauth2_client: BasicClient,
    http_client: reqwest::Client,
}

impl DiscordOauth2Repo {
    pub fn new(client_id: String, client_secret: String, redirect_url: String) -> Self {
        let auth_url = String::from("https://discord.com/api/oauth2/authorize?response_type=code");
        let token_url = String::from("https://discord.com/api/oauth2/token");

        let oauth2_client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new(auth_url).unwrap(),
            Some(TokenUrl::new(token_url).unwrap()),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap());

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(4))
            .build()
            .unwrap();

        Self {
            oauth2_client,
            http_client,
        }
    }

    pub fn redirect(&self) -> impl IntoResponse {
        let (auth_url, _csrf_token) = self
            .oauth2_client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("identify".to_string()))
            .url();

        Redirect::to(&auth_url.to_string())
    }

    pub async fn authenticate(&self, code: String) -> Result<NonZeroU64, String> {
        let token = self
            .oauth2_client
            .exchange_code(AuthorizationCode::new(code))
            .request_async(async_http_client)
            .await
            .map_err(|e| e.to_string())?;

        println!(
            "discord token expiry: {:?}",
            token.expires_in().map(|d| d.as_secs() / 3600)
        );
        println!(
            "discord refresh token: {:?}",
            token.refresh_token().map(|r| r.secret())
        );

        // https://discord.com/developers/docs/resources/user#user-object-user-structure
        #[derive(Debug, Deserialize)]
        struct User {
            id: String,
            //username: String,
            //discriminator: String,
        }

        let user: User = self
            .http_client
            .get("https://discord.com/api/users/@me")
            .timeout(Duration::from_secs(5))
            .bearer_auth(token.access_token().secret())
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<User>()
            .await
            .map_err(|e| e.to_string())?;

        //println!("{:?}", user);

        let parsed = user.id.parse::<u64>().map_err(|e| e.to_string())?;

        NonZeroU64::new(parsed).ok_or_else(|| String::from("discord id was 0"))
    }
}
