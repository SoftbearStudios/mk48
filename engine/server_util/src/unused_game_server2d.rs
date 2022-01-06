// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use common_util::unused_ticks::Ticks;

/// In the future, the game server (in addition to the game client) will satisfy a modular trait.
#[allow(dead_code)]
trait GameServer {
    // const MAX_WIDTH: i32;
    // type Terrain;

    fn start(&mut self, argv: [String]);
    fn stop();
    fn update(&mut self, ticks: Ticks);
}
