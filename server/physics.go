// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

import (
	"github.com/SoftbearStudios/mk48/server/world"
	"github.com/chewxy/math32"
	"math/rand"
	"runtime"
	"sync"
	"time"
)

func (h *Hub) Physics(ticks world.Ticks) {
	defer h.timeFunction("physics", time.Now())

	timeDeltaSeconds := min(ticks.Float(), 1.0)

	{
		terrain := world.Terrain(h.terrain)

		var outputWait sync.WaitGroup

		// For boats that die during iteration.
		boatOutput := make(chan world.Entity, runtime.NumCPU())
		deadBoats := make([]world.Entity, 0, 4)

		// For sculpting do on hub goroutine.
		type sculpt struct {
			position world.Vec2f // where to sculpt
			delta    float32     // how much land to add/remove
		}

		sculptOutput := make(chan sculpt, runtime.NumCPU())
		sculptPositions := make([]sculpt, 0, 8)

		outputWait.Add(1)
		go func() {
			for boat := range boatOutput {
				deadBoats = append(deadBoats, boat)
			}
			outputWait.Done()
		}()

		outputWait.Add(1)
		go func() {
			for pos := range sculptOutput {
				sculptPositions = append(sculptPositions, pos)
			}
			outputWait.Done()
		}()

		// Update movement and record various outputs
		h.world.SetParallel(true)
		h.world.ForEntities(func(e *world.Entity) (_, remove bool) {
			remove = e.Update(ticks, h.worldRadius, terrain)
			data := e.Data()
			if remove {
				if data.Kind == world.EntityKindBoat {
					boatOutput <- *e // Copy entity
				} else if data.Kind == world.EntityKindWeapon && (data.SubKind == world.EntitySubKindTorpedo || data.SubKind == world.EntitySubKindMissile || data.SubKind == world.EntitySubKindShell) {
					// This torpedo died of "natural" causes, affect the
					// terrain (see #49)

					// it's dangerous to add land at weapon impact point, as it may
					// be under an enemy ship, unfairly killing them
					/*
						if rand.Float32()*2 < data.Damage {
							// conserve land by adding some further along the path
							sculptOutput <- sculpt{
								position: e.Position.AddScaled(e.Direction.Vec2f(), 20),
								delta:    32 * data.Damage,
							}
						}
					*/

					if rand.Float32() < data.Damage {
						sculptOutput <- sculpt{
							position: e.Position.AddScaled(e.Direction.Vec2f(), 5),
							delta:    -8 * data.Damage,
						}
					}
				}
			} else if data.SubKind == world.EntitySubKindDredger {
				sculptOutput <- sculpt{
					position: e.Position,
					delta:    -5,
				}
			}
			return
		})
		h.world.SetParallel(false)

		close(boatOutput)
		close(sculptOutput)

		outputWait.Wait()

		for i := range deadBoats {
			h.boatDied(&deadBoats[i])
		}

		for _, s := range sculptPositions {
			h.terrain.Sculpt(s.position, s.delta)
		}
	}

	// Update entity to entity things such as collisions
	h.world.ForEntitiesAndOthers(func(entity *world.Entity) (stop bool, radius float32) {
		// Collectibles don't collide with each other
		if entity.Data().Kind == world.EntityKindCollectible {
			return
		}

		// Only test collisions with equal or smaller entities
		radius = entity.Data().Radius * 2

		// Unless the entity needs to know about its neighbors
		switch entity.Data().Kind {
		case world.EntityKindAircraft, world.EntityKindWeapon:
			radius = max(radius, entity.Data().Sensors.MaxRange())
		}

		return
	}, func(entity *world.Entity, other *world.Entity) (stop, remove, removeOther bool) {
		// Don't do friendly check, to allow team members to collide (See #27)
		entityData := entity.Data()
		otherData := other.Data()
		friendly := entity.Owner.Friendly(other.Owner)
		altitudeOverlap := entity.AltitudeOverlap(other)

		// Only do collision once when concurrent
		//if entityData.Radius < otherData.Radius || (entityData.Radius == otherData.Radius && entityID > otherEntityID) {
		//	return
		//}

		// Collisions are resolved by identifying the collision signature
		// i.e. the EntityKind of entities that are colliding
		var weapon, boat, otherBoat, collectible, decoy, obstacle *world.Entity

		if entityData.Kind == world.EntityKindBoat {
			boat = entity
		}
		if otherData.Kind == world.EntityKindBoat {
			if boat == nil {
				boat = other
			} else {
				otherBoat = other
			}
		}

		if entityData.Kind == world.EntityKindWeapon || entityData.Kind == world.EntityKindAircraft {
			weapon = entity
		} else if otherData.Kind == world.EntityKindWeapon || otherData.Kind == world.EntityKindAircraft {
			weapon = other
		}

		if entityData.Kind == world.EntityKindObstacle {
			obstacle = entity
		} else if otherData.Kind == world.EntityKindObstacle {
			obstacle = other
		}

		if otherData.Kind == world.EntityKindCollectible {
			collectible = other
		}

		// e must be either entity or other
		removeEntity := func(e *world.Entity, reason world.DeathReason) {
			data := e.Data()

			if data.Kind == world.EntityKindBoat {
				e.Owner.DeathReason = reason
				h.boatDied(e)
			}

			if e == entity {
				remove = true
			} else {
				removeOther = true
			}
		}

		if !entity.Collides(other, timeDeltaSeconds) || !altitudeOverlap {
			if collectible != nil && altitudeOverlap {
				// Collectibles gravitate towards players (except if they player paid them)
				if boat != nil && (boat.Owner != collectible.Owner || collectible.Ticks > 5*world.TicksPerSecond) {
					collectible.Direction = collectible.Direction.Lerp(boat.Position.Sub(collectible.Position).Angle(), timeDeltaSeconds*5)
					collectible.Velocity = 20 * world.MeterPerSecond
				}

				// Payment gravitates towards oil rigs
				if obstacle != nil && obstacle.EntityType == world.EntityTypeOilPlatform && collectible.Owner != nil {
					collectible.Direction = collectible.Direction.Lerp(obstacle.Position.Sub(collectible.Position).Angle(), timeDeltaSeconds)
					collectible.Velocity = 10 * world.MeterPerSecond
				}
			}

			if !friendly {
				// Mines do too
				if boat != nil && weapon != nil && altitudeOverlap && weapon.Data().SubKind == world.EntitySubKindMine {
					const attractDist = 40
					normal := boat.Direction.Vec2f()
					tangent := normal.Rot90()
					normalDistance := math32.Abs(normal.Dot(boat.Position) - normal.Dot(weapon.Position))
					tangentDistance := math32.Abs(tangent.Dot(boat.Position) - tangent.Dot(weapon.Position))
					if normalDistance < attractDist+boat.Data().Length*0.5 && tangentDistance < attractDist+boat.Data().Width*0.5 {
						weapon.Direction = weapon.Direction.Lerp(boat.Position.Sub(weapon.Position).Angle(), timeDeltaSeconds*5)
						weapon.Velocity = 5 * world.MeterPerSecond
					}
				}

				if entityData.Kind == world.EntityKindWeapon {
					// Home towards target/decoy
					if altitudeOverlap && entityData.Sensors.Any() {
						entity.UpdateSensor(other)
					}
				}

				// Aircraft/ASROC (simulate weapons and anti-aircraft)
				asroc := entityData.SubKind == world.EntitySubKindRocket && len(entityData.Armaments) > 0
				if (entityData.Kind == world.EntityKindAircraft || asroc) && otherData.Kind == world.EntityKindBoat {
					// Small window of opportunity to fire
					// Uses lifespan as torpedo consumption
					if (entity.Ticks > world.TicksPerSecond*3*world.Ticks(len(entityData.Armaments)) || asroc) && entity.Collides(other, 1.7+otherData.Length*0.01+entity.Hash()*0.5) {
						entity.Ticks = 0

						armaments := entityData.Armaments
						for i := range armaments {
							armamentData := &armaments[i]

							armament := &world.Entity{
								EntityType: armamentData.Type,
								Owner:      entity.Owner,
								Ticks:      armamentData.Type.ReducedLifespan(10 * world.TicksPerSecond),
								Transform:  entity.ArmamentTransform(i),
								Guidance: world.Guidance{
									DirectionTarget: entity.DirectionTarget + world.ToAngle((rand.Float32()-0.5)*0.1),
									VelocityTarget:  armamentData.Type.Data().Speed,
								},
							}

							const maxDropVelocity = 40 * world.MeterPerSecond
							if armament.Velocity > maxDropVelocity {
								armament.Velocity = maxDropVelocity
							}

							h.spawnEntity(armament, 0)
						}

						if asroc {
							// ASROC expires when dropping torpedo
							removeEntity(entity, world.DeathReason{})
						}
					}

					if otherData.AntiAircraft != 0 && entityData.Kind == world.EntityKindAircraft {
						d2 := entity.Position.DistanceSquared(other.Position)
						r2 := square(otherData.Radius * 1.5)

						// In range of aa
						if d2 < r2 {
							chance := (1.0 - d2/r2) * otherData.AntiAircraft
							if chance*timeDeltaSeconds > rand.Float32() {
								removeEntity(entity, world.DeathReason{})
							}
						}
					}
				}
			} else {
				if boat != nil && weapon != nil && boat.Owner == weapon.Owner &&
					weapon.Data().Kind == world.EntityKindAircraft &&
					weapon.Ticks > world.TicksPerSecond*5 && weapon.CanLand(boat) {

					// 0 ticks signals reload instantly.
					weapon.Ticks = 0
					removeEntity(weapon, world.DeathReason{})
				}
			}

			return
		}

		if entityData.Kind == world.EntityKindDecoy {
			decoy = entity
		} else if otherData.Kind == world.EntityKindDecoy {
			decoy = other
		}

		switch {
		case boat != nil && collectible != nil:
			// Players can collect the collectibles they paid...
			score := int(collectible.Data().Level)
			if boat.Data().SubKind == world.EntitySubKindTanker && collectible.EntityType == world.EntityTypeBarrel {
				score *= 2
			}
			boat.Owner.Score += score

			if boat.Owner != collectible.Owner {
				// ...but they don't repair or replenish them to avoid abuse
				boat.Repair(world.TicksPerSecond * 3 / 10)
				boat.Replenish(collectible.Data().Reload)
			}

			removeEntity(collectible, world.DeathReason{})
		case collectible != nil && collectible.Owner != nil && obstacle != nil && obstacle.EntityType == world.EntityTypeOilPlatform:
			// Payment upgrades oil rigs to HQs
			if rand.Float64() < 0.1 {
				obstacle.EntityType = world.EntityTypeHQ
				obstacle.Ticks = 0 // reset expiry
			}

			removeEntity(collectible, world.DeathReason{})
		case boat != nil && weapon != nil && !friendly:
			dist2 := entity.Position.DistanceSquared(other.Position)
			r2 := square(boat.Data().Radius)

			if boat.Damage(world.DamageToTicks(weapon.Data().Damage * collisionMultiplier(dist2, r2) * boat.SpawnProtection())) {
				weapon.Owner.Score += 10 + boat.Owner.Score/4
				removeEntity(boat, world.DeathReason{
					Type:   world.DeathTypeSinking,
					Player: weapon.Owner.Name,
					Entity: weapon.EntityType,
				})
			}

			removeEntity(weapon, world.DeathReason{})
		case boat != nil && otherBoat != nil:
			/*
				Goals:
				- (Cancelled) At least one boat is guaranteed to receive fatal damage
				- Ships with near equal max health and near equal health
				  percentage both die (no seemingly arbitrary survivor)
				- Low health boats still do damage, hence scale health percent
			*/

			baseDamage := timeDeltaSeconds * 1.1 * min((boat.HealthPercent()*0.5+0.5)*boat.MaxHealth().Damage(), (otherBoat.HealthPercent()*0.5+0.5)*otherBoat.MaxHealth().Damage())

			if friendly {
				baseDamage = 0
			}

			// Process boats both orders (each time acting only on the first boat, b)
			for _, ordering := range [2][2]*world.Entity{{boat, otherBoat}, {otherBoat, boat}} {
				b := ordering[0]
				oB := ordering[1]

				d := b.Data()
				oD := oB.Data()

				posDiff := b.Position.Sub(oB.Position).Norm()

				// Approximate mass
				m := d.Width * d.Length
				oM := oD.Width * oD.Length
				massDiff := oM / m

				if baseDamage > 0 {
					const ramDamage = 3
					damage := baseDamage

					// Colliding with center of boat is more deadly
					frontPos := oB.Position.AddScaled(oB.Direction.Vec2f(), oD.Length*0.5)
					dist2 := frontPos.DistanceSquared(b.Position)
					damage *= collisionMultiplier(dist2, square(d.Radius))
					damage *= b.SpawnProtection()

					// Rams take less damage from ramming
					isRam := d.SubKind == world.EntitySubKindRam
					if isRam {
						b.ClearSpawnProtection()
						massDiff *= 0.5
						damage *= 1.0 / ramDamage
					}

					// Rams give more damage while ramming
					isOtherRam := oD.SubKind == world.EntitySubKindRam
					if isOtherRam {
						massDiff *= 2
						damage *= ramDamage
					}

					if b.Damage(world.DamageToTicks(damage)) {
						deathType := world.DeathTypeCollision
						if isOtherRam {
							deathType = world.DeathTypeRamming
						}
						removeEntity(b, world.DeathReason{
							Type:   deathType,
							Player: oB.Owner.Name,
							Entity: oB.EntityType,
						})
					}
				}

				b.Velocity = b.Velocity.AddClamped(6*posDiff.Dot(b.Direction.Vec2f())*massDiff, 15*world.MeterPerSecond)
			}
		case boat != nil && obstacle != nil:
			posDiff := boat.Position.Sub(obstacle.Position).Norm()
			boat.Velocity = boat.Velocity.AddClamped(6*posDiff.Dot(boat.Direction.Vec2f()), 30*world.MeterPerSecond)
			if boat.KillIn(ticks, 6*world.TicksPerSecond) {
				removeEntity(boat, world.DeathReason{
					Type:   world.DeathTypeCollision,
					Entity: obstacle.EntityType,
				})
			}
		case boat != nil && decoy != nil:
			// No-op
		case weapon != nil && collectible != nil && collectible.EntityType == world.EntityTypeCoin:
			// No-op
			//
			// Don't allow coins (possibly placed by players) to interfere
			// with enemy weapons
		case !friendly:
			// Other ex weapon vs. weapon collision
			if entityData.Kind != world.EntityKindObstacle {
				removeEntity(entity, world.DeathReason{})
			}
			if otherData.Kind != world.EntityKindObstacle {
				removeEntity(other, world.DeathReason{})
			}
		}

		return
	})
}

// boatDied removes score and spawns crates
func (h *Hub) boatDied(e *world.Entity) {
	// Lose 1/2 score if you die
	// Cap at 50 so can't get max level right away
	e.Owner.Score /= 2
	if e.Owner.Score > 80 {
		e.Owner.Score = 80
	}

	data := e.Data()

	// Loot is based on the length of the boat
	loot := data.Length * 0.25 * (rand.Float32()*0.1 + 0.9)

	for i := 0; i < int(loot); i++ {
		lootType := world.EntityTypeScrap
		switch data.SubKind {
		case world.EntitySubKindPirate:
			if rand.Float32() < 0.5 {
				lootType = world.EntityTypeCoin
			}
		case world.EntitySubKindTanker:
			if rand.Float32() < 0.5 {
				lootType = world.EntityTypeBarrel
			}
		}

		crate := &world.Entity{
			EntityType: lootType,
			Transform:  e.Transform,
		}

		// Make loot roughly conform to rectangle of ship
		normal := e.Direction.Vec2f()
		tangent := normal.Rot90()
		crate.Position = crate.Position.AddScaled(normal, (rand.Float32()-0.5)*data.Length)
		crate.Position = crate.Position.AddScaled(tangent, (rand.Float32()-0.5)*data.Width)

		h.spawnEntity(crate, data.Radius*0.15)
	}
}

func collisionMultiplier(d2, r2 float32) float32 {
	return clamp(max(r2-d2+90, 0)/r2, 0.5, 1.5)
}
