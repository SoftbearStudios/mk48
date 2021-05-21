// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"fmt"
	"mk48/server/cloud"
	"mk48/server/terrain"
	"mk48/server/terrain/compressed"
	"mk48/server/terrain/noise"
	"mk48/server/world"
	"mk48/server/world/sector"
	"os"
	"sync/atomic"
	"time"
)

const (
	botPeriod         = time.Second / 4
	debugPeriod       = time.Second * 5
	leaderboardPeriod = time.Second
	spawnPeriod       = leaderboardPeriod
	updatePeriod      = world.TickPeriod

	// encodeBotMessages makes BotClient.Send marshal json and check for errors.
	// Only useful for testing/benchmarking (drops performance significantly).
	encodeBotMessages = false
)

// Hub maintains the set of active clients and broadcasts messages to the clients.
type Hub struct {
	// World state
	world       *sector.World
	worldRadius float32 // interpolated
	terrain     terrain.Terrain
	clients     ClientList // implemented as double-linked list
	despawn     ClientList // clients that are being removed
	teams       map[world.TeamID]*Team

	// Flags
	minPlayers int
	auth       string

	// Cloud (and things that are served atomically by HTTP)
	cloud      *cloud.Cloud
	statusJSON atomic.Value

	// chats are buffered until next update.
	chats []Chat
	// funcBenches are benchmarks of core Hub functions.
	funcBenches []funcBench

	// Inbound channels
	inbound    chan SignedInbound
	register   chan Client
	unregister chan Client

	// Timer based events
	cloudTicker       *time.Ticker
	updateTicker      *time.Ticker
	updateCounter     int
	updateTime        time.Time
	leaderboardTicker *time.Ticker
	debugTicker       *time.Ticker
	botsTicker        *time.Ticker
}

func newHub(minPlayers int, auth string) *Hub {
	c, err := cloud.New()
	if err != nil {
		fmt.Println("Cloud error:", err)
	}
	fmt.Println(c)

	radius := max(world.MinRadius, world.RadiusOf(minPlayers))
	return &Hub{
		cloud:             c,
		world:             sector.New(radius),
		terrain:           compressed.New(noise.NewDefault()),
		worldRadius:       radius,
		teams:             make(map[world.TeamID]*Team),
		minPlayers:        minPlayers,
		auth:              auth,
		inbound:           make(chan SignedInbound, 16+minPlayers*2),
		register:          make(chan Client, 8+minPlayers/256),
		unregister:        make(chan Client, 16+minPlayers/128),
		cloudTicker:       time.NewTicker(cloud.UpdatePeriod),
		updateTicker:      time.NewTicker(updatePeriod),
		updateTime:        time.Now(),
		leaderboardTicker: time.NewTicker(leaderboardPeriod),
		debugTicker:       time.NewTicker(debugPeriod),
		botsTicker:        time.NewTicker(botPeriod),
	}
}

func (h *Hub) run() {
	defer func() {
		if r := recover(); r != nil {
			panic(r)
		}
		println("That's it, I'm out -hub") // Don't waste time debugging hub exists
		os.Exit(1)
	}()

	h.Cloud()

	for {
		select {
		case client := <-h.register:
			h.clients.Add(client)
			client.Data().Hub = h
			client.Init()

			if _, bot := client.(*BotClient); !bot {
				h.cloud.IncrementPlayerStatistic()
			}
		case client := <-h.unregister:
			client.Close()
			player := &client.Data().Player.Player

			// Player no longer is joining teams
			// May want to do this during despawn because clearing team requests in O(n).
			h.clearTeamRequests(player)

			// Removes team or transfers ownership, if applicable
			h.leaveTeam(player)

			client.Data().Hub = nil
			h.clients.Remove(client)

			// Remove in Despawn during leaderboard update.
			h.despawn.Add(client)
		case in := <-h.inbound:
			// Read all messages currently in the channel
			n := len(h.inbound)

			for {
				// If not same hub the message is old
				data := in.Client.Data()
				if h == data.Hub {
					in.Inbound(h, in.Client, &data.Player)
				}

				if n--; n <= 0 {
					break
				}

				in = <-h.inbound
			}
		case <-h.updateTicker.C:
			now := time.Now()
			timeDelta := now.Sub(h.updateTime) + updatePeriod/10 // Kludge factor
			h.updateTime = now

			// Falling behind skip tick
			if timeDelta%updatePeriod > updatePeriod/5 {
				break
			}

			ticks := world.Ticks(timeDelta / updatePeriod)
			h.Physics(ticks)
			h.Update()
		case <-h.leaderboardTicker.C:
			h.terrain.Repair()
			h.Despawn()
			h.Spawn()
			h.Leaderboard()

			h.worldRadius = world.Lerp(h.worldRadius, world.RadiusOf(h.clients.Len), 0.25)
			h.world.Resize(h.worldRadius)
		case <-h.debugTicker.C:
			h.Debug()
			h.SnapshotTerrain()
		case <-h.botsTicker.C:
			// Add as many as fit in the channel but don't block because it would deadlock
			for i := h.clients.Len + len(h.register) - len(h.unregister); i < h.minPlayers; i++ {
				select {
				case h.register <- &BotClient{}:
				default:
					break
				}
			}
		case <-h.cloudTicker.C:
			h.Cloud()
		}
	}
}

func (h *Hub) clearTeamRequests(player *world.Player) {
	for _, team := range h.teams {
		team.JoinRequests.Remove(player)
	}
}

// Removes a player from the team that they are on. If the player was the owner,
// transfers or deletes the team depending on if there are remaining members
func (h *Hub) leaveTeam(player *world.Player) {
	if team := h.teams[player.TeamID]; team != nil {
		team.Members.Remove(player)

		// Team is empty, delete it
		if len(team.Members) == 0 {
			delete(h.teams, player.TeamID)
		}
	}

	player.TeamID = world.TeamIDInvalid
}
