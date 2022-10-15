// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::options::Options;

pub(crate) fn init_logger(options: &Options) {
    let mut logger = env_logger::builder();
    logger.format_timestamp(None);
    logger.filter_module("server", options.debug_game);
    logger.filter_module("game_server", options.debug_game);
    logger.filter_module("game_server::system", options.debug_watchdog);
    logger.filter_module("core_protocol", options.debug_core);
    logger.filter_module("server_util::web_socket", options.debug_sockets);
    logger.filter_module("server_util::linode", options.debug_watchdog);
    logger.filter_module("server_util::ssl", options.debug_watchdog);
    logger.init();
}
