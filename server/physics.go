// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"fmt"
	"github.com/chewxy/math32"
	"math/rand"
	"mk48/server/world"
	"runtime"
	"sync"
	"time"
)

func (h *Hub) Physics(ticks world.Ticks) {
	defer h.timeFunction("physics", time.Now())

	timeDeltaSeconds := min(ticks.Float(), 1.0)

	{
		terrain := world.Collider(h.terrain)

		var outputWait sync.WaitGroup

		// For boats that die during iteration.
		boatOutput := make(chan world.Entity, runtime.NumCPU())
		deadBoats := make([]world.Entity, 0, 4)

		// For sculpting do on hub goroutine.
		sculptOutput := make(chan world.Vec2f, runtime.NumCPU())
		sculptPositions := make([]world.Vec2f, 0, 8)

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
			if e.Data().Kind == world.EntityKindBoat {
				if remove {
					boatOutput <- *e // Copy entity
				} else if e.Data().SubKind == world.EntitySubKindDredger {
					sculptOutput <- e.Position
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

		for _, pos := range sculptPositions {
			h.terrain.Sculpt(pos, -5)
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
		if entity.Data().Kind == world.EntityKindWeapon {
			for _, sensor := range entity.Data().Sensors {
				radius = max(radius, sensor.Range)
			}
		}

		return
	}, func(entity *world.Entity, other *world.Entity) (stop, remove, removeOther bool) {
		// Don't do friendly check, to allow team members to collide (See #27)
		if entity.Owner == other.Owner {
			return
		}
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

		if entityData.Kind == world.EntityKindWeapon {
			weapon = entity
		} else if otherData.Kind == world.EntityKindWeapon {
			weapon = other
		}

		if otherData.Kind == world.EntityKindCollectible {
			collectible = other
		}

		// e must be either entity or other
		removeEntity := func(e *world.Entity, reason string) {
			data := e.Data()

			if data.Kind == world.EntityKindBoat {
				e.Owner.DeathMessage = reason
				h.boatDied(e)
			}

			if e == entity {
				remove = true
			} else {
				removeOther = true
			}
		}

		if !entity.Collides(other, timeDeltaSeconds) {
			// Collectibles gravitate towards players
			if boat != nil && collectible != nil && altitudeOverlap {
				collectible.Direction = collectible.Direction.Lerp(boat.Position.Sub(collectible.Position).Angle(), timeDeltaSeconds*5)
				collectible.Velocity = 20 * world.MeterPerSecond
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
					if altitudeOverlap && len(entityData.Sensors) > 0 && (otherData.Kind == world.EntityKindBoat || otherData.Kind == world.EntityKindDecoy) {
						entity.UpdateSensor(other)
					}

					// Aircraft (simulate weapons and anti-aircraft)
					if entityData.SubKind == world.EntitySubKindAircraft && otherData.Kind == world.EntityKindBoat {
						// Small window of opportunity to fire
						// Uses lifespan as torpedo consumption
						if entity.Lifespan > world.TicksPerSecond*3 && entity.Collides(other, 1.7+otherData.Length*0.01+entity.Hash()*0.5) {
							entity.Lifespan = 0
							torpedoType := world.EntityTypeMark18

							torpedo := &world.Entity{
								EntityType: torpedoType,
								Owner:      entity.Owner,
								Lifespan:   torpedoType.ReducedLifespan(10 * world.TicksPerSecond),
								Transform:  entity.Transform,
								Guidance: world.Guidance{
									DirectionTarget: entity.DirectionTarget + world.ToAngle((rand.Float32()-0.5)*0.1),
									VelocityTarget:  torpedoType.Data().Speed,
								},
							}

							h.spawnEntity(torpedo, 0)
						}

						if otherData.AntiAircraft != 0 {
							d2 := entity.Position.DistanceSquared(other.Position)
							r2 := square(otherData.Radius * 1.5)

							// In range of aa
							if d2 < r2 {
								chance := (1.0 - d2/r2*0.75) * otherData.AntiAircraft
								if chance*timeDeltaSeconds > rand.Float32() {
									removeEntity(entity, "shot down")
								}
							}
						}
					}
				}
			}

			return
		}

		if !altitudeOverlap {
			return
		}

		if entityData.Kind == world.EntityKindDecoy {
			decoy = entity
		} else if otherData.Kind == world.EntityKindDecoy {
			decoy = other
		}

		if entityData.Kind == world.EntityKindObstacle {
			obstacle = entity
		} else if otherData.Kind == world.EntityKindObstacle {
			obstacle = other
		}

		switch {
		case boat != nil && collectible != nil:
			// All collectibles have these benefits
			boat.Repair(0.05)
			if collectible.EntityType == world.EntityTypeCrate {
				// Prevent oil platforms from allowing infinite ammo
				boat.Replenish(1)
			}

			boat.Owner.Score += 1

			removeEntity(collectible, "collected")
		case boat != nil && weapon != nil && !friendly:
			damageMultiplier := boat.RecentSpawnFactor()

			dist2 := entity.Position.DistanceSquared(other.Position)
			r2 := square(boat.Data().Radius)
			damageMultiplier *= collisionMultiplier(dist2, r2)

			if boat.Damage(weapon.Data().Damage * damageMultiplier) {
				weapon.Owner.Score += 10 + boat.Owner.Score/4
				removeEntity(boat, fmt.Sprintf("Sunk by %s with a %s!", weapon.Owner.Name, weapon.Data().SubKind.Label()))
			}

			removeEntity(weapon, "hit")
		case boat != nil && otherBoat != nil:
			/*
				Goals:
				- (Cancelled) At least one boat is guaranteed to receive fatal damage
				- Ships with near equal max health and near equal health
				  percentage both die (no seemingly arbitrary survivor)
				- Low health boats still do damage, hence scale health percent
			*/

			baseDamage := timeDeltaSeconds * 1.1 * min((boat.HealthPercent()*0.5+0.5)*boat.MaxHealth(), (otherBoat.HealthPercent()*0.5+0.5)*otherBoat.MaxHealth())

			baseDamage *= boat.RecentSpawnFactor() * otherBoat.RecentSpawnFactor()

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

					// Rams take less damage from ramming
					isRam := d.SubKind == world.EntitySubKindRam
					if isRam {
						massDiff *= 0.5
						damage *= 1.0 / ramDamage
					}

					// Rams give more damage while ramming
					isOtherRam := oD.SubKind == world.EntitySubKindRam
					if isOtherRam {
						massDiff *= 2
						damage *= ramDamage
					}

					if b.Damage(damage) {
						verb := "Crashed into"
						if isOtherRam {
							verb = "Rammed by"
						}
						removeEntity(b, fmt.Sprintf("%s %s!", verb, oB.Owner.Name))
					}
				}

				b.Velocity = b.Velocity.AddClamped(6*posDiff.Dot(b.Direction.Vec2f())*massDiff, 15*world.MeterPerSecond)
			}
		case boat != nil && obstacle != nil:
			posDiff := boat.Position.Sub(obstacle.Position).Norm()
			boat.Velocity = boat.Velocity.AddClamped(6*posDiff.Dot(boat.Direction.Vec2f()), 30*world.MeterPerSecond)
			if boat.Damage(timeDeltaSeconds * boat.MaxHealth() * 0.15) {
				removeEntity(boat, fmt.Sprintf("Crashed into %s!", obstacle.Data().Label))
			}
		case !(friendly || (boat != nil && decoy != nil)):
			// Other ex weapon vs. weapon collision
			if entityData.Kind != world.EntityKindObstacle {
				removeEntity(entity, fmt.Sprintf("Crashed into %s!", other.Data().Label))
			}
			if otherData.Kind != world.EntityKindObstacle {
				removeEntity(other, fmt.Sprintf("Crashed into %s!", entity.Data().Label))
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

	// Makes spawn killing less profitable
	loot *= e.RecentSpawnFactor()

	for i := 0; i < int(loot); i++ {
		crate := &world.Entity{
			EntityType: world.EntityTypeCrate,
			Transform:  e.Transform,
		}

		h.spawnEntity(crate, data.Radius*0.5)
	}
}

func collisionMultiplier(d2, r2 float32) float32 {
	return clamp(max(r2-d2+90, 0)/r2, 0.5, 1.5)
}
