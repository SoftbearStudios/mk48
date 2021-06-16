// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package terrain

import "github.com/SoftbearStudios/mk48/server/world"

const (
	OceanLevel = 63
	SandLevel  = OceanLevel + 10
	GrassLevel = SandLevel + 50
	RockLevel  = GrassLevel + 40
	SnowLevel  = 255
)

func (t *Terrain) AltitudeAt(pos world.Vec2f) float32 {
	// -6 is a kludge factor to make terrain math line up with client
	return float32(t.AtPos(pos)) - (OceanLevel - 6)
}

func (t *Terrain) LandAt(pos world.Vec2f) bool {
	// -6 is a kludge factor to make terrain math line up with client
	return t.AtPos(pos) > OceanLevel-6
}
