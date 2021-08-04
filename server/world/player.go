// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"errors"
	"strconv"
	"strings"
	"time"
	"unsafe"
)

const (
	PlayerIDInvalid     = PlayerID(0)
	PlayerNameLengthMin = 3
	PlayerNameLengthMax = 12

	playerDeathTime     = time.Second * 2
	teamRespawnCooldown = time.Second * 10 // only affects respawning after dying at the hands of a player
)

type (
	// Player owns entities and has score
	Player struct {
		ext unsafeExtension // extension for EntityID
		PlayerData
		DeathReason     DeathReason
		DeathPos        Vec2f
		DeathTime       int64
		DeathVisual     float32 // if non-zero, in respawn animation
		DeathFromPlayer bool
		EntityID        EntityID
	}

	// PlayerID is the unique id of a Player
	PlayerID uintptr

	PlayerData struct {
		Name   string `json:"name,omitempty"`
		Score  int    `json:"score,omitempty"`
		TeamID TeamID `json:"team,omitempty"`
	}

	IDPlayerData struct {
		PlayerData
		PlayerID PlayerID `json:"playerID,omitempty"`
	}
)

func (player *Player) Friendly(other *Player) bool {
	return player == other || (player != nil && other != nil && player.TeamID != TeamIDInvalid && player.TeamID == other.TeamID)
}

func (player *Player) PlayerID() PlayerID {
	return PlayerID(unsafe.Pointer(player))
}

func (player *Player) IDPlayerData() IDPlayerData {
	var data IDPlayerData
	data.PlayerData = player.PlayerData
	data.PlayerID = player.PlayerID()
	return data
}

func (player *Player) Died(entity *Entity) {
	player.DeathPos, player.DeathVisual, _, _ = entity.Camera()
	player.DeathTime = unixMillis()
}

// Returns how long a player has been dead for in millis. Only valid for dead players.
func (player *Player) DeathDuration() int64 {
	return unixMillis() - player.DeathTime
}

// Respawning returns if player is currently in the respawn animation
func (player *Player) Respawning() bool {
	if player.DeathVisual != 0 {
		if player.DeathDuration() > int64(playerDeathTime/time.Millisecond) {
			player.ClearRespawn()
		} else {
			return true
		}
	}
	return false
}

// ClearRespawn clears after-death camera
func (player *Player) ClearRespawn() {
	// Do not clear death position/time/pvp. They are useful for implementing a
	// team respawn coodown. Clearing death visual is enough to end the respawn
	// animation
	player.DeathVisual = 0
}

// Clears all death related fields
func (player *Player) ClearDeath() {
	player.DeathVisual = 0
	player.DeathReason = DeathReason{}
	player.DeathTime = 0
	player.DeathPos = Vec2f{}
}

// Says nothing about whether player is in a team. Only whether they
// are allowed to respawn with it if it exists.
func (player Player) CanRespawnWithTeam() bool {
	return !player.DeathReason.FromPlayer() || unixMillis()-player.DeathTime > int64(teamRespawnCooldown/time.Millisecond)
}

// Camera Returns the camera that a player has if their entity doesn't exist
func (player *Player) Camera() (pos Vec2f, visual, radar, sonar float32) {
	if player.Respawning() {
		pos = player.DeathPos

		// Show players everything they would have seen if they had all the
		// sensors so they gain a better understanding of why they died
		visual = player.DeathVisual
		sonar = player.DeathVisual
		radar = player.DeathVisual
	} else {
		// Center of world (pos = 0,0) for title screen

		if player.DeathTime != 0 && player.DeathDuration() > 1000*60 {
			// Save (expensive) bandwidth.
			visual = 300
		} else {
			visual = 600
		}
	}
	return
}

var playerIDInvalidErr = errors.New("invalid player id")

func (playerID PlayerID) MarshalText() ([]byte, error) {
	return playerID.AppendText(make([]byte, 0, strconv.IntSize/4)), nil
}

func (playerID PlayerID) AppendText(buf []byte) []byte {
	if playerID == PlayerIDInvalid {
		panic(playerIDInvalidErr.Error())
	}
	return strconv.AppendUint(buf, uint64(playerID), 16)
}

func (playerID *PlayerID) UnmarshalText(text []byte) error {
	i, err := strconv.ParseUint(string(text), 16, strconv.IntSize)
	*playerID = PlayerID(i)
	if err == nil && *playerID == PlayerIDInvalid {
		err = playerIDInvalidErr
	}
	return err
}

func (data *PlayerData) ScoreLess(other *PlayerData) bool {
	if data.Score != other.Score {
		return data.Score > other.Score
	}
	if data.TeamID != other.TeamID {
		return data.TeamID < other.TeamID
	}
	return data.Name < other.Name
}

// Formats player first as: [team] name (score)
func (data PlayerData) String() string {
	var builder strings.Builder

	scoreString := strconv.Itoa(data.Score)
	n := len(data.Name) + len(scoreString) + 3

	if data.TeamID != TeamIDInvalid {
		teamID, _ := data.TeamID.MarshalText()
		n += len(teamID) + 3
		builder.Grow(n)

		builder.WriteByte('[')
		builder.Write(teamID)
		builder.WriteString("] ")
	} else {
		builder.Grow(n)
	}

	builder.WriteString(data.Name)

	builder.WriteString(" (")
	builder.WriteString(scoreString)
	builder.WriteByte(')')

	return builder.String()
}
