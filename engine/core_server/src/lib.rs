// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(once_cell)]
#![feature(binary_heap_into_iter_sorted)]
#![feature(bool_to_option)]
/*
 * Copyright (c) 2020 Softbear Studios - All Rights Reserved
 */
pub mod admin;
pub mod app;
mod arena;
mod chat;
pub mod client;
pub mod core;
mod database;
mod generate_id;
mod health;
mod invitation;
mod metrics;
mod notify_set;
mod repo;
pub mod server;
mod session;
mod team;
mod user_agent;
