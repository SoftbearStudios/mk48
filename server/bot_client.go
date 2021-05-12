// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"github.com/chewxy/math32"
	"io"
	"math/rand"
	"mk48/server/terrain"
	"mk48/server/world"
	"time"
)

type (
	BotClient struct {
		ClientData
		terrain       terrain.Terrain
		aggression    float32
		home          world.Vec2f // where bot will head towards if no other objective
		levelAmbition uint8       // max level to upgrade to
		destroying    bool
		request       int64 // last time requested team in millis
	}

	// Target is a contact that is closest or furthest
	Target struct {
		*Contact
		distanceSquared float32
	}
)

func (bot *BotClient) Close() {}

func (bot *BotClient) Data() *ClientData {
	return &bot.ClientData
}

func (bot *BotClient) Destroy() {
	if bot.destroying {
		return // In case goroutine hasn't run yet
	}

	bot.destroying = true
	hub := bot.Hub

	// Needs to go through always.
	select {
	case hub.unregister <- bot:
	default:
		go func() {
			hub.unregister <- bot
		}()
	}
}

func (bot *BotClient) Init(t terrain.Terrain) {
	bot.terrain = t

	r := getRand()

	bot.aggression = square(r.Float32())
	bot.levelAmbition = uint8(r.Intn(int(world.EntityLevelMax)) + 1)
	bot.spawn(r)

	poolRand(r)
}

func (bot *BotClient) Send(out outbound) {
	if bot.destroying {
		return
	}

	if encodeBotMessages {
		// Discard output
		if err := json.NewEncoder(io.Discard).Encode(Message{Data: out}); err != nil {
			panic("bot test marshal: " + err.Error())
		}
	}

	// Use local rand to avoid locking
	r := getRand()

	switch data := out.(type) {
	case *Update:
		if data.EntityID == world.EntityIDInvalid {
			if prob(r, 0.25) {
				bot.Destroy() // rage quit
				return
			} else {
				// Leave team if die
				if prob(r, 0.5) {
					bot.receiveAsync(RemoveFromTeam{
						PlayerID: data.PlayerID,
					})
				}
				bot.spawn(r)
			}
			return
		}

		var ship Contact
		for i := range data.Contacts {
			if data.Contacts[i].EntityID == data.EntityID {
				ship = data.Contacts[i].Contact
			}
		}

		// Create or leave team
		if prob(r, 1e-4) {
			if ship.TeamID == world.TeamIDInvalid {
				bot.receiveAsync(CreateTeam{
					Name: randomTeamName(r),
				})
			} else if prob(r, 0.5) {
				bot.receiveAsync(RemoveFromTeam{
					PlayerID: data.PlayerID,
				})
			}
		}

		for _, request := range data.TeamRequests {
			diff := float64(ship.Score - request.Score)

			// Only accept members with similar score
			if prob(r, 1.0/(50.0+(diff*diff)*0.05)) {
				bot.receiveAsync(AddToTeam{
					PlayerID: request.PlayerID,
				})
			}
		}

		now := unixMillis()
		requesting := now-bot.request > int64(time.Second*5/time.Millisecond) // in milliseconds

		var closestEnemy, closestCollectible, closestHazard Target

		for i := range data.Contacts {
			contact := &data.Contacts[i].Contact
			if contact.Friendly {
				continue
			}

			distanceSquared := ship.Position.DistanceSquared(contact.Position)
			data := contact.EntityType.Data()

			if data.Kind == world.EntityKindBoat {
				closestEnemy.Closest(contact, distanceSquared)
			}

			if data.Kind == world.EntityKindCollectible {
				closestCollectible.Closest(contact, distanceSquared)
			} else if !(data.Kind == world.EntityKindBoat && ship.EntityType.Data().SubKind == world.EntitySubKindRam) {
				// Rams don't regard boats as hazards
				closestHazard.Closest(contact, distanceSquared)
			}

			// Join teams that have more score most of the time for protection
			if requesting && ship.TeamID == world.TeamIDInvalid && contact.TeamID != world.TeamIDInvalid &&
				((ship.Score < contact.Score-5 && prob(r, 2e-3)) || prob(r, 1e-4)) {

				bot.request = now
				requesting = false

				bot.receiveAsync(AddToTeam{
					TeamID: contact.TeamID,
				})
			}
		}

		shipData := ship.EntityType.Data()

		if (bot.home == world.Vec2f{}) || ship.Position.DistanceSquared(bot.home) < 100*100 {
			// Pick a new random home
			bot.home = world.Angle(r.Float32() * math32.Pi * 2).Vec2f().Mul(data.WorldRadius * 0.9)
		}

		manual := Manual{
			EntityID: data.EntityID,
			Guidance: world.Guidance{
				VelocityTarget:  10,
				DirectionTarget: bot.home.Sub(ship.Position).Angle(),
			},
		}

		if shipData.SubKind == world.EntitySubKindSubmarine {
			altitudeTarget := float32(-1)
			manual.AltitudeTarget = &altitudeTarget
		}

		if closestCollectible.Found() {
			manual.VelocityTarget = 20
			manual.DirectionTarget = closestCollectible.Position.Sub(ship.Position).Angle()
		}

		if closestEnemy.Found() && closestEnemy.distanceSquared < 2*closestCollectible.distanceSquared {
			closestEnemyAngle := closestEnemy.Position.Sub(ship.Position).Angle()

			manual.VelocityTarget = closestEnemy.Velocity + 10
			manual.DirectionTarget = closestEnemyAngle

			manual.TurretTarget = new(world.Vec2f)
			*manual.TurretTarget = closestEnemy.Position

			if prob(r, float64(bot.aggression)) {
				bestArmamentIndex := -1
				bestArmamentAngleDiff := world.Angle(math32.MaxFloat32)

				for index, armament := range shipData.Armaments {
					armamentType := armament.Type
					if armamentType == world.EntityKindInvalid {
						armamentType = armament.Default.Data().Kind
					}

					armamentSubtype := armament.Subtype
					if armamentSubtype == world.EntitySubKindInvalid {
						armamentSubtype = armament.Default.Data().SubKind
					}

					if armamentType == world.EntityKindWeapon {
						if world.HasArmament(ship.ArmamentConsumption, index) {
							armamentTransform := world.ArmamentTransform(ship.EntityType, ship.Transform, ship.TurretAngles, index)
							diff := closestEnemyAngle.Diff(armamentTransform.Direction).Abs()
							if diff < bestArmamentAngleDiff {
								bestArmamentIndex = index
								bestArmamentAngleDiff = diff
							}
						}
					}
				}

				if bestArmamentIndex != -1 && closestEnemy.distanceSquared < square(4*shipData.Length) && bestArmamentAngleDiff < world.Angle(math32.Pi/3) {
					bot.receiveAsync(Fire{
						Index: bestArmamentIndex,
						Guidance: world.Guidance{
							DirectionTarget: closestEnemyAngle + 0.25*world.Angle(r.Float64()-0.5),
						},
					})
				}
			}
		}

		if inFront := ship.Position.AddScaled(ship.Direction.Vec2f(), shipData.Length*2); bot.terrain.AtPos(inFront) > terrain.OceanLevel-6 {
			manual.VelocityTarget = 5
			manual.DirectionTarget = ship.Direction + world.Angle(math32.Pi/2)
		} else if closestHazard.Found() && closestHazard.distanceSquared < square(closestHazard.EntityType.Data().Length+shipData.Length*2) {
			manual.VelocityTarget = 10
			manual.DirectionTarget = closestHazard.Position.Sub(ship.Position).Angle().Inv()
		} else if shipData.Level < bot.levelAmbition {
			if upgradePaths := ship.EntityType.UpgradePaths(ship.Score); len(upgradePaths) > 0 {
				bot.receiveAsync(Upgrade{
					Type: randomType(r, upgradePaths),
				})
			}
		}

		bot.receiveAsync(manual)
	}

	// Pool resources
	poolRand(r)
	out.Pool()
}

// receiveAsync Doesn't deadlock the hub
func (bot *BotClient) receiveAsync(in inbound) {
	select {
	case bot.Hub.inbound <- SignedInbound{Client: bot, inbound: in}:
	default:
		// Drop bot messages to avoid downfall of server
	}
}

func (bot *BotClient) spawn(r *rand.Rand) {
	name := bot.Player.Name
	if name == "" {
		name = randomBotName(r)
	}

	bot.receiveAsync(Spawn{
		Type: randomType(r, world.SpawnEntityTypes),
		Name: name,
	})
}

func randomType(r *rand.Rand, entityTypes []world.EntityType) world.EntityType {
	return entityTypes[r.Intn(len(entityTypes))]
}

func (t *Target) Closest(contact *Contact, distanceSquared float32) {
	if t.Contact == nil || distanceSquared < t.distanceSquared {
		t.Contact = contact
		t.distanceSquared = distanceSquared
	}
}

func (t *Target) Found() bool {
	return t.Contact != nil
}
