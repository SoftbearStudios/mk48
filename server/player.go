// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"github.com/SoftbearStudios/mk48/server/world"
)

// Player is an extension of world.Player with extra data
type Player struct {
	world.Player
	ChatHistory ChatHistory
	FPS         float32

	// Optimizations
	TerrainArea world.AABB
}
