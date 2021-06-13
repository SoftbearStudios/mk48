// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

const (
	DeathTypeBorder    = "border"    // Never Player or Entity
	DeathTypeTerrain   = "terrain"   // Never Player or Entity
	DeathTypeCollision = "collision" // requires Player or Entity
	DeathTypeRamming   = "ramming"   // requires Player
	DeathTypeSinking   = "sinking"   // requires Player and Entity
)

// Has a custom (but regular) jsoniter marshaler
type DeathReason struct {
	Type   string     `json:"message,omitempty"`
	Player string     `json:"player,omitempty"`
	Entity EntityType `json:"entity,omitempty"`
}

// Returns whether the death was a result of player actions
func (reason *DeathReason) FromPlayer() bool {
	return reason.Player != ""
}
