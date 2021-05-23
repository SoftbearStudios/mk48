// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"mk48/server/world"
	"runtime"
	"sync"
	"time"
)

// Update sends an Update message to each Client.
// It's run in parallel because it doesn't write to World
func (h *Hub) Update() {
	defer h.timeFunction("update", time.Now())

	cpus := runtime.NumCPU()
	if cpus > 1 && h.world.SetParallel(true) {
		var wait sync.WaitGroup
		wait.Add(cpus)
		input := make(chan Client, cpus*2)

		for i := 0; i < cpus; i++ {
			go func(hub *Hub, in <-chan Client, wg *sync.WaitGroup) {
				j := 0
				for c := range in {
					// 1/8 of clients get a forced terrain update each time
					hub.updateClient(c, (hub.updateCounter+j)%8 == 0)
					j++
				}
				wg.Done()
			}(h, input, &wait)
		}

		for client := h.clients.First; client != nil; client = client.Data().Next {
			input <- client
		}

		close(input)
		wait.Wait()

		h.world.SetParallel(false)
	} else {
		j := 0
		for client := h.clients.First; client != nil; client = client.Data().Next {
			h.updateClient(client, (h.updateCounter+j)%8 == 0)
			j++
		}
	}

	// chats have been sent, reset the buffer
	// Cannot reuse slice because would cause data race
	h.chats = nil
	for _, team := range h.teams {
		team.Chats = nil
	}

	h.updateCounter++
}

// Sends an Update to a Client containing contacts, chat, and team info.
// Can be safely called concurrently once per client/player.
func (h *Hub) updateClient(client Client, forceSendTerrain bool) {
	update := NewUpdate()
	p := &client.Data().Player
	player := &p.Player

	update.EntityID = player.EntityID
	update.PlayerID = player.PlayerID()
	update.DeathMessage = player.DeathMessage
	update.WorldRadius = h.worldRadius
	update.Chats = h.chats

	h.world.EntityByID(player.EntityID, func(ship *world.Entity) (_ bool) {
		var visualRange float32
		var radarRange float32
		var sonarRange float32
		var position world.Vec2f

		if ship != nil {
			position, visualRange, radarRange, sonarRange = ship.Camera()
		} else {
			position, visualRange, radarRange, sonarRange = p.Camera()
		}

		maxRange := max(visualRange, max(radarRange, sonarRange))

		// Make subsequent math more efficient by squaring and inverting
		visualRangeInv := invSquare(visualRange)
		radarRangeInv := invSquare(radarRange)
		sonarRangeInv := invSquare(sonarRange)

		h.world.ForEntitiesInRadius(position, maxRange, func(distanceSquared float32, entity *world.Entity) (_ bool) {
			known := entity.Owner == player || (distanceSquared < 800*800 && entity.Owner.Friendly(player))
			alt := entity.Altitude()

			// visible means contact's EntityType, HealthPercent, ArmamentConsumption, and TurretAngles are known
			var visible bool
			// uncertainty is the amount of error of the sensor
			var uncertainty float32

			if !known {
				data := entity.Data()

				invSize := data.InvSize // cached 1.0 / min(1, data.Radius*(1.0/50.0)*(1-data.Stealth))
				defaultRatio := distanceSquared * invSize
				uncertainty = 1.0

				if radarRangeInv != 0 && alt >= -0.1 {
					radarRatio := defaultRatio * radarRangeInv
					uncertainty = min(uncertainty, radarRatio*2)
				}

				if sonarRangeInv != 0 && alt <= 0 {
					sonarRatio := defaultRatio * sonarRangeInv
					uncertainty = min(uncertainty, sonarRatio*3)
				}

				if visualRangeInv != 0 {
					visualRatio := defaultRatio * visualRangeInv
					if alt < 0.0 {
						visualRatio /= clamp(alt+1.0, 0.05, 1)
					}
					visible = visualRatio < 1
					uncertainty = min(uncertainty, visualRatio)
				}

				if uncertainty >= 1.0 {
					return
				}
			}

			// Grow slice if too small
			if contacts := update.Contacts; len(contacts) == cap(contacts) {
				update.Contacts = append(contacts, IDContact{})[:len(contacts)]
			}

			// Ensure space for new contact
			n := len(update.Contacts)
			update.Contacts = update.Contacts[:n+1]
			c := &update.Contacts[n]

			c.Uncertainty = uncertainty
			c.Transform = entity.Transform
			c.EntityID = entity.EntityID
			c.EntityType = entity.EntityType
			c.Altitude = alt

			if known || visible {
				if entity.Data().Kind == world.EntityKindBoat {
					// Both slices are copy on write so don't have to copy on read
					c.ArmamentConsumption = entity.ArmamentConsumption()
					c.TurretAngles = entity.TurretAngles()

					c.Damage = entity.DamagePercent()
				}

				// You only know the Guidance of allies
				if known {
					c.Guidance = entity.Guidance
				}
			}

			if c.Uncertainty < 0.75 && entity.Owner != nil {
				c.Friendly = entity.Owner.Friendly(player)
				c.IDPlayerData = entity.Owner.IDPlayerData()
			}

			return
		})

		// Only send terrain to real players for now
		if _, ok := client.(*SocketClient); ok {
			terrainPos := position.Sub(world.Vec2f{X: visualRange, Y: visualRange})
			aabb := world.AABBFrom(terrainPos.X, terrainPos.Y, visualRange*2, visualRange*2)

			// If terrain changed
			if clamped := h.terrain.Clamp(aabb); p.TerrainArea != clamped || forceSendTerrain {
				p.TerrainArea = clamped
				update.Terrain = h.terrain.At(aabb)
			}
		}

		return
	})

	if team := h.teams[player.TeamID]; team != nil {
		update.TeamChats = team.Chats
		update.TeamMembers = team.Members.AppendData(update.TeamMembers)

		// Only team owner gets the requests
		if player == team.Owner() {
			update.TeamCode = team.Code
			update.TeamRequests = team.JoinRequests.AppendData(update.TeamRequests)
		}
	}

	// Client pools update when its done with it
	client.Send(update)
}
