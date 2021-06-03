// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

import (
	"fmt"
	"github.com/SoftbearStudios/mk48/server/cloud"
	"github.com/SoftbearStudios/mk48/server/terrain"
	"github.com/SoftbearStudios/mk48/server/terrain/compressed"
	"github.com/SoftbearStudios/mk48/server/terrain/noise"
	"github.com/SoftbearStudios/mk48/server/world"
	"github.com/SoftbearStudios/mk48/server/world/sector"
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

	// Must spawn atleast this many bots per real player,
	// to give low-level ships some easier targets
	minBotRatio = 0.5

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
	minPlayers       int
	botMaxSpawnLevel uint8
	auth             string

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
	skippedCounter    int
	updateCounter     int
	leaderboardTicker *time.Ticker
	debugTicker       *time.Ticker
	botsTicker        *time.Ticker
}

func NewHub(minPlayers int, botMaxSpawnLevel int, auth string) *Hub {
	c, err := cloud.New()
	if err != nil {
		fmt.Println("Cloud error:", err)
	}
	fmt.Println(c)

	if botMaxSpawnLevel > int(world.BoatLevelMax) {
		botMaxSpawnLevel = int(world.BoatLevelMax)
	}

	radius := max(world.MinRadius, world.RadiusOf(minPlayers))
	return &Hub{
		cloud:             c,
		world:             sector.New(radius),
		terrain:           compressed.New(noise.NewDefault()),
		worldRadius:       radius,
		teams:             make(map[world.TeamID]*Team),
		minPlayers:        minPlayers,
		botMaxSpawnLevel:  uint8(botMaxSpawnLevel),
		auth:              auth,
		inbound:           make(chan SignedInbound, 16+minPlayers*2),
		register:          make(chan Client, 8+minPlayers/256),
		unregister:        make(chan Client, 16+minPlayers/128),
		cloudTicker:       time.NewTicker(cloud.UpdatePeriod),
		updateTicker:      time.NewTicker(updatePeriod),
		leaderboardTicker: time.NewTicker(leaderboardPeriod),
		debugTicker:       time.NewTicker(debugPeriod),
		botsTicker:        time.NewTicker(botPeriod),
	}
}

func (h *Hub) Run() {
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

			if !client.Bot() {
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
		case updateTime := <-h.updateTicker.C:
			now := time.Now()
			if missed := now.Sub(updateTime) - updatePeriod/10; missed > 0 {
				h.skippedCounter += int(missed/updatePeriod) + 1
				break
			}

			// Physics would start to break down after this.
			const maxTicksPerUpdate = 4
			if h.skippedCounter > maxTicksPerUpdate {
				fmt.Println("server behind more than", maxTicksPerUpdate, "ticks:", h.skippedCounter)
				h.skippedCounter = maxTicksPerUpdate
			}

			ticks := world.Ticks(h.skippedCounter) + 1
			h.skippedCounter = 0

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
			// There are two reasons to add bots:
			// - When minPlayers is not met by bots + clients
			// - When minBotRatio is not met by bots / clients
			playerCount := 0
			for client := h.clients.First; client != nil; client = client.Data().Next {
				if !client.Bot() {
					playerCount++
				}
			}

			botCount := h.clients.Len - playerCount
			minBots := int(float32(playerCount) * minBotRatio)
			totalClients := h.clients.Len + len(h.register) - len(h.unregister)

			// Add as many as fit in the channel but don't block because it would deadlock
			for i := totalClients; i < h.minPlayers || botCount < minBots; i++ {
				select {
				case h.register <- &BotClient{}:
					botCount++
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
