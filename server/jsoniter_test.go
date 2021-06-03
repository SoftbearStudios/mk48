// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"bytes"
	"github.com/SoftbearStudios/mk48/server/world"
	"testing"
)

func TestJsonIter(t *testing.T) {
	const testPlayerID = 0x963fa0
	const testEntityID = 0xffff

	var teamID world.TeamID
	err := teamID.UnmarshalText([]byte("foo"))
	if err != nil {
		t.Errorf(err.Error())
	}

	testPlayerData := world.PlayerData{Name: "bob", Score: 2, TeamID: teamID}
	testUpdate := Message{Data: &Update{
		Chats: []Chat{{Message: "hi", PlayerData: testPlayerData}},
		Contacts: []IDContact{{EntityID: testEntityID, Contact: Contact{
			Guidance:            world.Guidance{DirectionTarget: 0.0},
			IDPlayerData:        world.IDPlayerData{PlayerID: testPlayerID, PlayerData: testPlayerData},
			Transform:           world.Transform{Position: world.Vec2f{X: 1.0, Y: 0.5}, Velocity: world.ToVelocity(0.1875), Direction: world.ToAngle(0.25)},
			ArmamentConsumption: []world.Ticks{0, world.TicksPerSecond / 10, world.TicksPerSecond / 5, world.TicksPerSecond},
			Friendly:            false,
			EntityType:          world.ParseEntityType("fairmileD"),
			Uncertainty:         0.2,
		}}},
		EntityID: testEntityID,
		PlayerID: testPlayerID,
	}}

	const testUpdateString = `{"data":{"chats":[{"name":"bob","score":2,"team":"foo","message":"hi"}],"contacts":{"ffff":{"name":"bob","score":2,"team":"foo","playerID":"963fa0","position":{"x":1,"y":0.5},"velocity":0.1875,"direction":0.249943,"armamentConsumption":[0,0.1,0.2,1],"type":"fairmileD","uncertainty":0.2}},"playerID":"963fa0","entityID":"ffff"},"type":"update"}`

	buf, err := json.Marshal(testUpdate)
	if err != nil {
		t.Error("error marshaling:", err.Error())
		return
	}
	if !bytes.Equal(buf, []byte(testUpdateString)) {
		j := -1
		for i := range buf {
			a := buf[i]
			b := testUpdateString[i]
			if a != b {
				j = i
				break
			}
		}
		t.Error("different output:\none:", testUpdateString, "\ntwo:", string(buf), "\none len:", len(testUpdateString),
			"\ntwo len:", len(buf), "\ndiff:", j, "\none:", testUpdateString[j:], "\ntwo:", string(buf[j:]))
	}

	realPlayerID := world.PlayerID(0xffaa1234)
	const playerIDString = `{"playerID": "ffaa1234"}`

	var playerIDWrapper struct {
		PlayerID world.PlayerID `json:"playerID"`
	}
	err = json.Unmarshal([]byte(playerIDString), &playerIDWrapper)
	playerID := playerIDWrapper.PlayerID
	if err != nil {
		t.Error("error unmarshaling:", err.Error())
		return
	}
	if playerID != realPlayerID {
		t.Error("different output:\nexpected:", realPlayerID, "\ngot:", playerID, "\n")
	}

	realEntityID := world.EntityID(0x123abc)
	const entityIDString = `{"entityID": "123abc"}`

	var entityIDWrapper struct {
		EntityID world.EntityID `json:"entityID"`
	}
	err = json.Unmarshal([]byte(entityIDString), &entityIDWrapper)
	entityID := entityIDWrapper.EntityID
	if err != nil {
		t.Error("error unmarshaling:", err.Error())
		return
	}
	if entityID != realEntityID {
		t.Error("different output:\nexpected:", realEntityID, "\ngot:", entityID, "\n")
	}

	realEntityType := world.ParseEntityType("fairmileD")
	const entityTypeString = `{"entityType": "fairmileD"}`

	var entityTypeWrapper struct {
		EntityType world.EntityType `json:"entityType"`
	}
	err = json.Unmarshal([]byte(entityTypeString), &entityTypeWrapper)
	entityType := entityTypeWrapper.EntityType
	if err != nil {
		t.Error("error unmarshaling:", err.Error())
		return
	}
	if entityType != realEntityType {
		t.Error("different output:\nexpected:", realEntityType, "\ngot:", entityType, "\n")
	}
}
