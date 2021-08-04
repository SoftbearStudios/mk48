// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

import (
	"github.com/SoftbearStudios/mk48/server/world"
	"github.com/chewxy/math32"
	"runtime"
	"sync"
	"time"
)

func (h *Hub) shouldForceSendTerrain(client Client) bool {
	player := &client.Data().Player
	mod := 10

	// Save bandwidth for dead players
	if player.DeathTime != 0 {
		duration := player.DeathDuration()
		if duration > 5*60*1000 {
			mod = 500
		} else if duration > 60*1000 {
			mod = 100
		} else {
			mod = 20
		}
	}

	return (h.updateCounter+int(player.PlayerID()))%mod == 0
}

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
					hub.updateClient(c, h.shouldForceSendTerrain(c))
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
			h.updateClient(client, h.shouldForceSendTerrain(client))
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
	update.DeathReason = player.DeathReason
	update.WorldRadius = h.worldRadius
	update.Chats = h.chats

	h.world.EntityByID(player.EntityID, func(ship *world.Entity) (_ bool) {
		var visualRange float32
		var radarRange float32
		var sonarRange float32
		var position world.Vec2f
		var active bool
		var absVel float32

		if ship == nil {
			active = true
			position, visualRange, radarRange, sonarRange = p.Camera()
		} else {
			active = ship.Active()

			absVel = math32.Abs(ship.Velocity.Float())
			position, visualRange, radarRange, sonarRange = ship.Camera()
		}

		maxRange := max(visualRange, max(radarRange, sonarRange))

		// Make subsequent math more efficient by squaring and inverting
		visualRangeInv := invSquare(visualRange)
		radarRangeInv := invSquare(radarRange)
		sonarRangeInv := invSquare(sonarRange)

		h.world.ForEntitiesInRadius(position, maxRange, func(distanceSquared float32, entity *world.Entity) (_ bool) {
			friendly := entity.Owner.Friendly(player)
			known := entity.Owner == player || (distanceSquared < 800*800 && friendly)
			alt := entity.Altitude()
			data := entity.Data()

			// visible means contact's EntityType, HealthPercent, ArmamentConsumption, and TurretAngles are known
			var visible bool
			// uncertainty is the amount of error of the sensor
			var uncertainty float32

			if !known {
				invSize := data.InvSize // cached 1.0 / min(1, data.Radius*(1.0/50.0)*(1-data.Stealth))
				defaultRatio := distanceSquared * invSize
				uncertainty = 1.0
				contactAbsVel := math32.Abs(entity.Velocity.Float())

				if radarRangeInv != 0 && alt >= -0.1 {
					radarRatio := defaultRatio * radarRangeInv

					if active {
						// Active radar can see moving targets easier
						uncertainty = min(uncertainty, radarRatio*15/(15+contactAbsVel))
					}

					// Passive radar
					emission := float32(5)
					if data.Kind == world.EntityKindBoat {
						emission += 5
						if entity.Active() && data.Sensors.Radar.Range > 0 {
							// Active radar gives away entity's position
							emission += 20
						}
					} else if data.SubKind == world.EntitySubKindMissile {
						emission += 30
					}

					radarRatio *= 25 / emission

					uncertainty = min(uncertainty, radarRatio)
				}

				if sonarRangeInv != 0 && alt <= 0 {
					sonarRatio := defaultRatio * sonarRangeInv
					if active {
						// Active sonar
						uncertainty = min(uncertainty, sonarRatio)
					}

					// Passive sonar
					if data.Kind == world.EntityKindBoat || data.Kind == world.EntityKindWeapon || data.Kind == world.EntityKindDecoy {
						// Can hear moving targets easier
						noise := max(contactAbsVel-5, 10)

						if data.Kind != world.EntityKindBoat {
							noise += 100
						} else if entity.Active() && data.Sensors.Sonar.Range > 0 {
							// Active sonar gives away entity's position
							noise += 20
						}
						sonarRatio /= noise
					}
					// Making noise of your own reduces the performance of
					// passive sonar
					sonarRatio *= 10 + absVel
					uncertainty = min(uncertainty, sonarRatio)
				}

				if visualRangeInv != 0 {
					visualRatio := defaultRatio * visualRangeInv
					if alt < 0.0 {
						visualRatio /= mapRanges(alt, -0.5, 1, 0.025, 0.8, true)
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
			if data.Kind == world.EntityKindCollectible || friendly || c.Uncertainty < 0.5 || distanceSquared < 100*100 {
				c.EntityType = entity.EntityType
			}
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

			if c.Uncertainty < 0.5 && entity.Owner != nil {
				c.Friendly = entity.Owner.Friendly(player)
				if data.Kind == world.EntityKindBoat {
					c.IDPlayerData = entity.Owner.IDPlayerData()
					team := h.teams[entity.Owner.TeamID]
					if team != nil {
						c.TeamFull = team.Full()

						// The following code lies that the team is full to
						// serve various purposes

						// Bots always leave 2 spots for real players
						c.TeamFull = c.TeamFull || (client.Bot() && len(team.Members)+len(team.JoinRequests) >= world.TeamMembersMax-2)

						// Already joining this team
						c.TeamFull = c.TeamFull || team.JoinRequests.Contains(player)
					}
				}
			}

			return
		})

		// Bot client doesn't need terrain data
		if _, ok := client.(*BotClient); !ok {
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
