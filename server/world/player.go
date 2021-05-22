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

	playerDeathTime = time.Second * 2
)

type (
	// Player owns entities and has score
	Player struct {
		ext unsafeExtension // extension for EntityID
		PlayerData
		DeathMessage string
		DeathPos     Vec2f
		DeathTime    int64
		DeathVisual  float32
		EntityID     EntityID
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

// Respawning returns if player is currently in the respawn animation
func (player *Player) Respawning() bool {
	if player.DeathTime != 0 {
		if unixMillis()-player.DeathTime > int64(playerDeathTime/time.Millisecond) {
			player.ClearRespawn()
		} else {
			return true
		}
	}
	return false
}

// ClearRespawn clears death camera
func (player *Player) ClearRespawn() {
	player.DeathPos = Vec2f{}
	player.DeathVisual = 0
	player.DeathTime = 0
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
		// Center of world for title screen
		visual = 1000
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
