// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package terrain

import "github.com/SoftbearStudios/mk48/server/world"

const (
	// Based on how many bits represent a terrain altitude
	resolution = 1 << 4

	// Start of sand layer (right above water)
	SandLevel = 255 / 2

	// Start of grass layer (right above sand)
	GrassLevel = SandLevel + 1*resolution
)

// Returns altitude (in meters) above sea level
func (t *Terrain) AltitudeAt(pos world.Vec2f) float32 {
	// 0.3 is a kludge factor
	return (float32(t.AtPos(pos)) - SandLevel) * 0.3
}

// Returns whether the position lies in land (sand or higher)
func (t *Terrain) LandAt(pos world.Vec2f) bool {
	return t.AtPos(pos) >= SandLevel
}
