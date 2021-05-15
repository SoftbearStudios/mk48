// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"fmt"
	"math/rand"
	"mk48/server/world"
	"runtime"
	"sync"
	"time"
)

// must have owner
func logDeath(entity *world.Entity) {
	if true {
		_ = AppendLog("/tmp/mk48-death.log", []interface{}{
			unixMillis(),
			entity.Owner.Name,
			entity.Owner.DeathMessage,
			entity.HealthPercent(),
		})
	}
}

func (h *Hub) Physics(timeDelta time.Duration) {
	defer h.timeFunction("physics", time.Now())

	timeDeltaSeconds := float32(timeDelta.Seconds())

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
		h.world.ForEntities(func(_ world.EntityID, e *world.Entity) (_, remove bool) {
			remove = e.Update(timeDeltaSeconds, h.worldRadius, terrain)
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
	h.world.ForEntitiesAndOthers(func(_ world.EntityID, entity *world.Entity) (stop bool, radius float32) {
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
	}, func(entityID world.EntityID, entity *world.Entity, otherEntityID world.EntityID, other *world.Entity) (stop, remove, removeOther bool) {
		// Don't do friendly check, to allow team members to collide (See #27)
		if entity.Owner == other.Owner || !entity.AltitudeOverlap(other) {
			return
		}
		entityData := entity.Data()
		otherData := other.Data()
		friendly := entity.Owner.Friendly(other.Owner)

		// Only do collision once when concurrent
		//if entityData.Radius < otherData.Radius || (entityData.Radius == otherData.Radius && entityID > otherEntityID) {
		//	return
		//}

		// Collisions are resolved by identifying the collision signature
		// i.e. the EntityKind of entities that are colliding
		var weapon, boat, otherBoat, collectible, obstacle *world.Entity

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

		if !entity.Collides(other, timeDeltaSeconds) {
			// Collectibles gravitate towards players
			if boat != nil && collectible != nil {
				collectible.Direction = collectible.Direction.Lerp(boat.Position.Sub(collectible.Position).Angle(), timeDeltaSeconds*5)
				collectible.Velocity = 20.0
			}

			// Home towards target
			if !friendly && entityData.Kind == world.EntityKindWeapon && len(entityData.Sensors) > 0 && otherData.Kind == world.EntityKindBoat {
				entity.UpdateSensor(other)
			}

			return
		}

		if entityData.Kind == world.EntityKindObstacle {
			obstacle = entity
		} else if otherData.Kind == world.EntityKindObstacle {
			obstacle = other
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

		switch {
		case boat != nil && collectible != nil:
			// All collectibles have these benefits
			boat.Repair(0.05)
			boat.Replenish(0.1)
			boat.Owner.Score += 1

			removeEntity(collectible, "collected")
		case boat != nil && weapon != nil && !friendly:
			damageMultiplier := boat.RecentSpawnFactor()

			dist2 := boat.Position.DistanceSquared(weapon.Position)
			r2 := square(boat.Data().Radius)
			damageMultiplier *= clamp(1.5*(r2-dist2)/r2, 0.5, 1.5)

			boat.Damage += weapon.Data().Damage * damageMultiplier

			if boat.Dead() {
				weapon.Owner.Score += 10 + boat.Owner.Score/4
				removeEntity(boat, fmt.Sprintf("Sunk by %s with a %s!", weapon.Owner.Name, weapon.Data().SubKind.Label()))
			}

			removeEntity(weapon, "hit")
		case boat != nil && otherBoat != nil:
			/*
				Goals:
				- (Cancelled) At least one boat is guaranteed to receive fatal damage
				- Ships with near equal max health and near equal health
				  percentage both die (no seemingly arbitrary survivor) hence 110%
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

				isRam := b.Data().SubKind == world.EntitySubKindRam
				isOtherRam := oB.Data().SubKind == world.EntitySubKindRam

				posDiff := b.Position.Sub(oB.Position).Norm()

				damage := baseDamage

				if isRam || isOtherRam {
					// Ouch
					damage *= 2
				}

				if !isRam || isOtherRam {
					b.Damage += damage
				}
				b.Velocity = clampMagnitude(b.Velocity+6*posDiff.Dot(b.Direction.Vec2f()), 15)

				if b.Dead() {
					verb := "Crashed into"
					if isOtherRam {
						verb = "Rammed by"
					}
					removeEntity(b, fmt.Sprintf("%s %s!", verb, oB.Owner.Name))
				}
			}
		case boat != nil && obstacle != nil:
			posDiff := boat.Position.Sub(obstacle.Position).Norm()
			boat.Velocity = clampMagnitude(boat.Velocity+6*posDiff.Dot(boat.Direction.Vec2f()), 30)
			boat.Damage += timeDeltaSeconds * boat.MaxHealth() * 0.15
			if boat.Dead() {
				removeEntity(boat, fmt.Sprintf("Crashed into %s!", obstacle.Data().Label))
			}
		case !friendly:
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
	logDeath(e)

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
