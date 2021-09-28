// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#![feature(once_cell)]
/*
 * Copyright (c) 2020 Softbear Studios - All Rights Reserved
 */
mod arena;
mod bot;
mod chat;
pub mod core;
mod database;
mod generate_id;
mod metrics;
mod notify_set;
mod repo;
mod session;
mod team;

#[macro_use]
extern crate lazy_static;
