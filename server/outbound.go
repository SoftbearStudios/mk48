// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	terrain2 "mk48/server/terrain"
	"mk48/server/world"
	"sync"
)

type (
	// Chat is a chat message.
	Chat struct {
		world.PlayerData
		Message string `json:"message"`
	}

	// Contact is a view of a world.Entity from an observer.
	// Guidance is not always set, but still marshaled as zeros.
	// TODO: Marshal fake data that represents observable angular velocity without revealing the exact target direction
	Contact struct {
		world.Guidance
		world.IDPlayerData
		world.Transform
		ArmamentConsumption []float32        `json:"armamentConsumption,omitempty"`
		TurretAngles        []world.Angle    `json:"turretAngles,omitempty"`
		Friendly            bool             `json:"friendly,omitempty"`
		EntityType          world.EntityType `json:"type"`
		Altitude            float32          `json:"altitude,omitempty"`
		Damage              float32          `json:"damage,omitempty"`
		Uncertainty         float32          `json:"uncertainty"`
	}

	// IDContact is a Contact paired with a world.EntityID for efficiency.
	IDContact struct {
		Contact
		world.EntityID
	}

	// Leaderboard is the top 10 players with the most score.
	Leaderboard struct {
		Leaderboard []world.PlayerData `json:"leaderboard"`
	}

	// Update is a view of Contacts, TeamMembers, and Terrain.
	// It is dependant special marshaller on Update.Contacts to marshal as a map.
	Update struct {
		Chats        []Chat               `json:"chats,omitempty"`
		Contacts     []IDContact          `json:"contacts,omitempty"`
		TeamChats    []Chat               `json:"teamChats,omitempty"`
		TeamCode     world.TeamCode       `json:"teamInvite,omitempty"`
		TeamMembers  []world.IDPlayerData `json:"teamMembers,omitempty"`
		TeamRequests []world.IDPlayerData `json:"teamJoinRequests,omitempty"`
		DeathMessage string               `json:"deathMessage,omitempty"`
		Terrain      *terrain2.Data       `json:"terrain,omitempty"`

		// Put smaller fields here for packing
		PlayerID    world.PlayerID `json:"playerID,omitempty"`
		EntityID    world.EntityID `json:"entityID,omitempty"`
		WorldRadius float32        `json:"worldRadius,omitempty"`
	}
)

func init() {
	registerOutbound(
		Leaderboard{},
		&Update{},
	)
}

const poolContactsCap = 32

var updatePool = sync.Pool{
	New: func() interface{} {
		return &Update{
			Contacts:    make([]IDContact, 0, poolContactsCap),
			TeamMembers: make([]world.IDPlayerData, 0, world.TeamMembersMax),
		}
	},
}

func NewUpdate() *Update {
	return updatePool.Get().(*Update)
}

// Pool Uses pointers for reuse in pool
func (update *Update) Pool() {
	// Use separate pool for terrain because not all updates have terrain
	if update.Terrain != nil {
		update.Terrain.Pool()
	}

	// Delete all fields except Contacts, TeamMembers, and TeamRequests
	*update = Update{
		Contacts:     clearIDContacts(update.Contacts),
		TeamMembers:  clearIDPlayerData(update.TeamMembers),
		TeamRequests: clearIDPlayerData(update.TeamRequests),
	}
	updatePool.Put(update)
}

func (leaderboard Leaderboard) Pool() {}

func clearIDContacts(contacts []IDContact) []IDContact {
	for i := range contacts {
		contacts[i] = IDContact{}
	}
	return contacts[:0]
}

func clearIDPlayerData(data []world.IDPlayerData) []world.IDPlayerData {
	for i := range data {
		data[i] = world.IDPlayerData{}
	}
	return data[:0]
}
