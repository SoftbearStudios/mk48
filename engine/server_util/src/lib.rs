// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(once_cell)]
#![feature(drain_filter)]
#![feature(associated_type_defaults)]

pub mod cloud;
pub mod database;
pub mod database_schema;
pub mod generate_id;
pub mod health;
pub mod http;
pub mod ip_rate_limiter;
pub mod linode;
pub mod notify_set;
pub mod observer;
pub mod os;
pub mod rate_limiter;
pub mod ssl;
pub mod user_agent;
