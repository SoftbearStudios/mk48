// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"encoding/binary"
	"errors"
	"math/rand"
	"strconv"
)

// No team
const (
	TeamCodeBase    = 36
	TeamCodeInvalid = TeamCode(0)
	TeamIDInvalid   = TeamID(0)
	TeamIDLengthMin = 1
	TeamIDLengthMax = 6
	TeamMembersMax  = 6
)

type (
	// PlayerSet Set with order
	PlayerSet []*Player

	// Team A group of players on the same team
	Team struct {
		JoinRequests PlayerSet
		Members      PlayerSet // First member is owner
		Code         TeamCode
	}

	// TeamCode is a code that allows a Player to join a Team.
	// Used with invite links.
	TeamCode uint32

	// TeamID is a fixed-length string Team name that needs to be unique
	// Use uint64 for fast comparisons and can store TeamIDLengthMax bytes
	TeamID uint64
)

func (team *Team) Create(owner *Player) {
	*team = Team{
		Code:    TeamCode(rand.Uint32()),
		Members: PlayerSet{owner}, // First member is owner
	}
}

func (set *PlayerSet) GetByID(playerID PlayerID) *Player {
	for _, p := range *set {
		if p.PlayerID() == playerID {
			return p
		}
	}
	return nil
}

func (set *PlayerSet) Remove(player *Player) {
	players := *set
	for i := range players {
		if players[i] == player {
			// Shift players over to maintain order
			copy(players[i:len(players)-1], players[i+1:])
			players = players[:len(players)-1]
			break
		}
	}
	*set = players
}

func (set *PlayerSet) Add(player *Player) {
	for _, p := range *set {
		if p == player {
			return // Already in set
		}
	}
	*set = append(*set, player)
}

// AppendData converts a PlayerSet to []IDPlayerData
// Uses append api to reuse old slice
func (set *PlayerSet) AppendData(buf []IDPlayerData) []IDPlayerData {
	if n := len(*set); cap(buf) < n {
		b := make([]IDPlayerData, len(buf), n)
		copy(b, buf)
		buf = b
	}

	for _, p := range *set {
		buf = append(buf, p.IDPlayerData())
	}
	return buf
}

// sort.Interface

func (set *PlayerSet) Len() int {
	return len(*set)
}

func (set *PlayerSet) Less(i, j int) bool {
	s := *set
	return s[i].ScoreLess(&s[j].PlayerData)
}

func (set *PlayerSet) Swap(i, j int) {
	h := *set
	h[i], h[j] = h[j], h[i]
}

// heap.Interface

func (set *PlayerSet) Push(x interface{}) {
	*set = append(*set, x.(*Player))
}

func (set *PlayerSet) Pop() interface{} {
	h := *set
	n := len(h) - 1
	x := h[n]
	h[n] = nil // Clear pointer
	h = h[:n]
	*set = h
	return x
}

// Owner First member of team is owner
func (team *Team) Owner() *Player {
	if len(team.Members) > 0 {
		return team.Members[0]
	}
	return nil
}

func (team *Team) Full() bool {
	return len(team.Members) >= TeamMembersMax
}

// TeamCode helpers

func (code TeamCode) String() string {
	return string(code.AppendText(make([]byte, 0, 8)))
}

var teamCodeInvalidErr = errors.New("invalid team code")

func (code TeamCode) MarshalText() ([]byte, error) {
	return code.AppendText(make([]byte, 0, 8)), nil
}

func (code TeamCode) AppendText(text []byte) []byte {
	if code == TeamCodeInvalid {
		panic(teamCodeInvalidErr.Error())
	}
	return strconv.AppendUint(text, uint64(code), TeamCodeBase)
}

func (code *TeamCode) UnmarshalText(text []byte) error {
	i, err := strconv.ParseUint(string(text), TeamCodeBase, 32)
	if err != nil {
		return err
	}

	*code = TeamCode(i)
	if *code == TeamCodeInvalid {
		return teamCodeInvalidErr
	}
	return nil
}

// TeamID helpers

func (teamID TeamID) String() string {
	return string(teamID.AppendText(make([]byte, 0, 8)))
}

var teamIDInvalidErr = errors.New("invalid player id")

func (teamID TeamID) MarshalText() ([]byte, error) {
	return teamID.AppendText(make([]byte, 0, 8)), nil
}

func (teamID TeamID) AppendText(text []byte) []byte {
	if teamID == TeamIDInvalid {
		panic(teamIDInvalidErr.Error())
	}

	buf := make([]byte, 8)
	binary.LittleEndian.PutUint64(buf, uint64(teamID))

	i := TeamIDLengthMin
	for ; i < TeamIDLengthMax; i++ {
		if buf[i] == 0 {
			break
		}
	}

	return append(text, buf[:i]...)
}

func (teamID *TeamID) UnmarshalText(text []byte) error {
	if len(text) < TeamIDLengthMin || len(text) > TeamIDLengthMax {
		return teamIDInvalidErr
	}

	buf := make([]byte, 8)
	copy(buf, text)

	*teamID = TeamID(binary.LittleEndian.Uint64(buf))
	if *teamID == TeamIDInvalid {
		return teamIDInvalidErr
	}
	return nil
}
