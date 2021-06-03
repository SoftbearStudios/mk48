// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

import (
	"github.com/SoftbearStudios/mk48/server/terrain"
	"github.com/SoftbearStudios/mk48/server/world"
	"github.com/chewxy/math32"
	"io"
	"math/rand"
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

func (bot *BotClient) Bot() bool {
	return true
}

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
	bot.levelAmbition = uint8(r.Intn(int(world.BoatLevelMax)) + 1)
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
		// Checks if bot ship does not exist
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

		// Get contact of bot's own ship with linear search once.
		// Map has higher overhead so we use a slice.
		var ship Contact
		for i := range update.Contacts {
			if update.Contacts[i].EntityID == update.EntityID {
				ship = update.Contacts[i].Contact
				break
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

		// Accept new team members
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
		var closestEnemy, closestFriendly, closestCollectible, closestHazard Target
		shipData := ship.EntityType.Data()

		// Scan sensor contacts
		for i := range update.Contacts {
			if update.Contacts[i].EntityID == update.EntityID {
				// Ignore self
				continue
			}

			contact := &update.Contacts[i].Contact
			distanceSquared := ship.Position.DistanceSquared(contact.Position)
			contactData := contact.EntityType.Data()

			if contactData.Kind == world.EntityKindCollectible {
				closestCollectible.Closest(contact, distanceSquared)
			} else if (!contact.Friendly || contactData.Kind == world.EntityKindBoat) && !(!contact.Friendly && contactData.Kind == world.EntityKindBoat && shipData.SubKind == world.EntitySubKindRam) {
				// Rams don't regard unfriendly boats as hazards
				closestHazard.Closest(contact, distanceSquared)
			}

			if contactData.Kind == world.EntityKindBoat {
				if contact.Friendly {
					friendDistance := distanceSquared
					if len(update.TeamMembers) > 0 && contact.Name == update.TeamMembers[0].Name {
						// Prioritize team leader
						friendDistance = 0
					}
					closestFriendly.Closest(contact, friendDistance)
				} else {
					closestEnemy.Closest(contact, distanceSquared)
				}
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

		// Prepare a manual steering command to send
		manual := Manual{
			EntityID: update.EntityID,
		}

		if shipData.SubKind == world.EntitySubKindSubmarine {
			// Stay submerged (TODO: This allocates on the heap)
			altitudeTarget := float32(-1)
			manual.AltitudeTarget = &altitudeTarget
		}

		// The purpose of this switch is to assign a value to
		//  - manual.VelocityTarget
		//  - manual.DirectionTarget
		switch {
		case bot.isLandInMultiDirection(ship.Position, shipData.Length, ship.Direction):
			// Avoid terrain by turning slowly.
			manual.VelocityTarget = 5 * world.MeterPerSecond
			manual.DirectionTarget = ship.Direction + world.Pi/2
		case closestHazard.Found() && closestHazard.distanceSquared < square(closestHazard.EntityType.Data().Length+shipData.Length*2):
			// Avoid collisions by turning away
			awayDirection := closestHazard.Position.Sub(ship.Position).Angle().Inv()

			if closestHazard.Friendly && closestHazard.EntityType.Data().Kind == world.EntityKindBoat {
				// Don't turn completely away
				manual.DirectionTarget = closestHazard.Direction.Lerp(awayDirection, 0.15)
				manual.VelocityTarget = closestHazard.Velocity
			} else {
				manual.DirectionTarget = awayDirection
				manual.VelocityTarget = 10 * world.MeterPerSecond
			}
		case closestFriendly.Found():
			// Wander towards closest friendly ship
			manual.DirectionTarget = closestFriendly.Position.Sub(ship.Position).Angle()
			manual.VelocityTarget = closestFriendly.Velocity + 5*world.MeterPerSecond
		case closestEnemy.Found():
			closestEnemyAngle := closestEnemy.Position.Sub(ship.Position).Angle()

			if closestEnemy.Velocity > 0 {
				// Prevent negative velocities from preventing chase
				manual.VelocityTarget = closestEnemy.Velocity
			}
			// Make velocity target dependent on aggresssion due to #87
			manual.VelocityTarget += world.ToVelocity(5 + bot.aggression*10)
			manual.DirectionTarget = closestEnemyAngle
		case closestCollectible.Found():
			manual.VelocityTarget = 20 * world.MeterPerSecond
			manual.DirectionTarget = closestCollectible.Position.Sub(ship.Position).Angle()
		default:
			// Wander to a random destination
			// Reset destination when it is reached
			if (bot.destination == world.Vec2f{}) || ship.Position.DistanceSquared(bot.destination) < 100*100 {
				// Pick a random destination to wander to.
				bot.destination = world.ToAngle(r.Float32() * math32.Pi * 2).Vec2f().Mul(update.WorldRadius * 0.9)
			}

			manual.DirectionTarget = bot.destination.Sub(ship.Position).Angle()
			manual.VelocityTarget = 10 * world.MeterPerSecond
		}

		// Upgrade up to level ambition if available.
		if shipData.Level < bot.levelAmbition {
			if upgradePaths := ship.EntityType.UpgradePaths(ship.Score); len(upgradePaths) > 0 {
				bot.receiveAsync(Upgrade{
					Type: randomType(r, upgradePaths),
				})
			}
		}

		// Attack with weapons (regardless of pathfinding)
		if closestEnemy.Found() {
			// Aim
			manual.TurretTarget = closestEnemy.Position

			// Fire
			if prob(r, float64(bot.aggression*0.1)) {
				closestEnemyAngle := closestEnemy.Position.Sub(ship.Position).Angle()
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

					if armamentType != world.EntityKindWeapon {
						continue
					}
					if armamentSubtype == world.EntitySubKindSAM {
						// TODO: Teach bots how to use SAMs
						continue
					}

					if ship.ArmamentConsumption[index] == 0 {
						armamentTransform := world.ArmamentTransform(ship.EntityType, ship.Transform, ship.TurretAngles, index)
						diff := closestEnemyAngle.Diff(armamentTransform.Direction).Abs()
						if armament.Vertical || armament.Default.Data().SubKind == world.EntitySubKindAircraft {
							diff = 0
						}
						if diff < bestArmamentAngleDiff {
							bestArmamentIndex = index
							bestArmamentAngleDiff = diff
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

	var level = r.Intn(int(bot.Hub.botMaxSpawnLevel)) + 1

	bot.receiveAsync(Spawn{
		Type: randomType(r, world.BoatEntityTypesByLevel[level]),
		Name: name,
	})
}

// Multiple checks of a range of angles
func (bot *BotClient) isLandInMultiDirection(pos world.Vec2f, length float32, angle world.Angle) bool {
	for i := float32(-0.5); i < 0.5; i += 0.1 {
		if bot.isLandInDirection(pos, length, angle+world.ToAngle(i)) {
			return true
		}
	}
	return false
}

func (bot *BotClient) isLandInDirection(pos world.Vec2f, length float32, angle world.Angle) bool {
	inFront := pos.AddScaled(angle.Vec2f(), length*2)

	// Regard world border as land for the purpose of bots
	if inFront.LengthSquared() > square(bot.Hub.worldRadius) {
		return true
	}

	// -6 is a kludge factor to make terrain math line up with client
	return bot.Hub.terrain.AtPos(inFront) > terrain.OceanLevel-6
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
