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
		destination   world.Vec2f // where bot will head towards if no other objective
		aggression    float32     // how likely bot is to attack when given a chance
		levelAmbition uint8       // max level to upgrade to
		destroying    bool        // already called destroy
		request       int64       // last time requested team in millis
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
	// In case goroutine hasn't run yet don't overload server by creating another one.
	if bot.destroying {
		return
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

func (bot *BotClient) Init() {
	r := getRand()
	defer poolRand(r)

	bot.aggression = square(r.Float32())
	bot.levelAmbition = uint8(r.Intn(int(world.EntityLevelMax)) + 1)
	bot.spawn(r)
}

func (bot *BotClient) Send(out outbound) {
	defer out.Pool()

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
	defer poolRand(r)

	switch update := out.(type) {
	case *Update:
		if update.EntityID == world.EntityIDInvalid {
			// When bot dies it either quits, leaves its team, or does nothing.
			if prob(r, 0.25) {
				bot.Destroy()
			} else {
				if prob(r, 0.5) {
					bot.receiveAsync(RemoveFromTeam{PlayerID: update.PlayerID})
				}
				bot.spawn(r)
			}
			return
		}

		// Get contact with linear search once.
		// Map has higher overhead so we use a slice.
		var ship Contact
		for i := range update.Contacts {
			if update.Contacts[i].EntityID == update.EntityID {
				ship = update.Contacts[i].Contact
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
					PlayerID: update.PlayerID,
				})
			}
		}

		// Accept new members
		// TODO(caibear) deny members
		for _, request := range update.TeamRequests {
			diff := float64(ship.Score - request.Score)

			// Only accept members with similar score
			if prob(r, 1.0/(50.0+(diff*diff)*0.05)) {
				bot.receiveAsync(AddToTeam{
					PlayerID: request.PlayerID,
				})
			}
		}

		// Only team request every 5 seconds
		now := unixMillis()
		requesting := now-bot.request > int64(time.Second*5/time.Millisecond) // in milliseconds

		// Find enemies, collectibles, and hazards.
		var closestEnemy, closestCollectible, closestHazard Target
		shipData := ship.EntityType.Data()

		for i := range update.Contacts {
			contact := &update.Contacts[i].Contact
			if contact.Friendly {
				continue
			}

			distanceSquared := ship.Position.DistanceSquared(contact.Position)
			contactData := contact.EntityType.Data()

			if contactData.Kind == world.EntityKindBoat {
				closestEnemy.Closest(contact, distanceSquared)
			}

			if contactData.Kind == world.EntityKindCollectible {
				closestCollectible.Closest(contact, distanceSquared)
			} else if !(contactData.Kind == world.EntityKindBoat && shipData.SubKind == world.EntitySubKindRam) {
				// Rams don't regard boats as hazards
				closestHazard.Closest(contact, distanceSquared)
			}

			// Favor joining teams that have more score for protection.
			if requesting && ship.TeamID == world.TeamIDInvalid && contact.TeamID != world.TeamIDInvalid &&
				((ship.Score < contact.Score-5 && prob(r, 2e-3)) || prob(r, 1e-4)) {

				bot.request = now
				requesting = false

				bot.receiveAsync(AddToTeam{
					TeamID: contact.TeamID,
				})
			}
		}

		// Pick a random destination to wander to.
		if (bot.destination == world.Vec2f{}) || ship.Position.DistanceSquared(bot.destination) < 100*100 {
			bot.destination = world.Angle(r.Float32() * math32.Pi * 2).Vec2f().Mul(update.WorldRadius * 0.9)
		}

		manual := Manual{
			EntityID: update.EntityID,
			Guidance: world.Guidance{
				VelocityTarget:  10 * world.MeterPerSecond,
				DirectionTarget: bot.destination.Sub(ship.Position).Angle(),
			},
		}

		if shipData.SubKind == world.EntitySubKindSubmarine {
			altitudeTarget := float32(-1)
			manual.AltitudeTarget = &altitudeTarget
		}

		if closestCollectible.Found() {
			manual.VelocityTarget = 20 * world.MeterPerSecond
			manual.DirectionTarget = closestCollectible.Position.Sub(ship.Position).Angle()
		}

		if closestEnemy.Found() && closestEnemy.distanceSquared < 2*closestCollectible.distanceSquared {
			closestEnemyAngle := closestEnemy.Position.Sub(ship.Position).Angle()

			manual.VelocityTarget = closestEnemy.Velocity + 10*world.MeterPerSecond
			manual.DirectionTarget = closestEnemyAngle

			manual.TurretTarget = closestEnemy.Position

			// Attack based on aggression.
			if prob(r, float64(bot.aggression)) {
				bestArmamentIndex := -1
				bestArmamentAngleDiff := float32(math32.MaxFloat32)

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
						if ship.ArmamentConsumption[index] == 0 {
							armamentTransform := world.ArmamentTransform(ship.EntityType, ship.Transform, ship.TurretAngles, index)
							diff := closestEnemyAngle.Diff(armamentTransform.Direction).Abs()
							if diff < bestArmamentAngleDiff {
								bestArmamentIndex = index
								bestArmamentAngleDiff = diff
							}
						}
					}
				}

				if bestArmamentIndex != -1 && closestEnemy.distanceSquared < square(4*shipData.Length) && bestArmamentAngleDiff < math32.Pi/3 {
					bot.receiveAsync(Fire{
						Index:          bestArmamentIndex,
						PositionTarget: closestEnemy.Position,
					})
				}
			}
		}

		if inFront := ship.Position.AddScaled(ship.Direction.Vec2f(), shipData.Length*2); bot.Hub.terrain.AtPos(inFront) > terrain.OceanLevel-6 {
			// Avoid terrain by turning slowly.
			manual.VelocityTarget = 5 * world.MeterPerSecond
			manual.DirectionTarget = ship.Direction + world.Pi/2
		} else if closestHazard.Found() && closestHazard.distanceSquared < square(closestHazard.EntityType.Data().Length+shipData.Length*2) {
			// Try to turn away from threats.
			manual.VelocityTarget = 10 * world.MeterPerSecond
			manual.DirectionTarget = closestHazard.Position.Sub(ship.Position).Angle().Inv()
		} else if shipData.Level < bot.levelAmbition {
			// Upgrade up to level ambition if available.
			if upgradePaths := ship.EntityType.UpgradePaths(ship.Score); len(upgradePaths) > 0 {
				bot.receiveAsync(Upgrade{
					Type: randomType(r, upgradePaths),
				})
			}
		}

		bot.receiveAsync(manual)
	}
}

// receiveAsync doesn't deadlock the hub.
func (bot *BotClient) receiveAsync(in inbound) {
	select {
	case bot.Hub.inbound <- SignedInbound{Client: bot, inbound: in}:
	default:
		// Drop bot messages to avoid downfall of server.
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
