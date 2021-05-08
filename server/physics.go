// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"fmt"
	"github.com/chewxy/math32"
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

		// For boats that die during iteration
		output := make(chan world.Entity, runtime.NumCPU())
		deadBoats := make([]world.Entity, 0, 4)
		var outputWait sync.WaitGroup
		outputWait.Add(1)

		go func() {
			for id := range output {
				deadBoats = append(deadBoats, id)
			}
			outputWait.Done()
		}()

		// Update movement and terrain
		h.world.SetParallel(true)
		h.world.ForEntities(func(_ world.EntityID, e *world.Entity) (_, remove bool) {
			remove = e.Update(timeDeltaSeconds, h.worldRadius, terrain)
			if e.Data().Kind == world.EntityKindBoat {
				if e.Data().SubKind == world.EntitySubKindDredger {
					h.terrain.Sculpt(e.Position, -5)
				}
				if remove {
					output <- *e // Copy entity
				}
			}
			return
		})
		h.world.SetParallel(false)

		close(output)
		outputWait.Wait()

		for i := range deadBoats {
			h.boatDied(&deadBoats[i])
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
		if entity.Owner.Friendly(other.Owner) || math32.Abs(entity.Altitude()-other.Altitude()) > world.AltitudeCollisionThreshold {
			return
		}
		entityData := entity.Data()
		otherData := other.Data()

		// Only do collision once when concurrent
		//if entityData.Radius < otherData.Radius || (entityData.Radius == otherData.Radius && entityID > otherEntityID) {
		//	return
		//}

		var weapon, boat, otherBoat, collectible *world.Entity

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

		if !entity.Collides(other, timeDeltaSeconds) || (weapon != nil &&
			weapon.Data().SubKind == world.EntitySubKindMissile && weapon.Distance < 150) {

			// Collectibles gravitate towards players
			if boat != nil && collectible != nil {
				collectible.Direction = collectible.Direction.Lerp(boat.Position.Sub(collectible.Position).Angle(), timeDeltaSeconds*5)
				collectible.Velocity = 20.0
			}

			// Home towards target
			if entityData.Kind == world.EntityKindWeapon && len(entityData.Sensors) > 0 && otherData.Kind == world.EntityKindBoat {
				entity.UpdateSensor(other)
			}

			return
		}

		// e must be entity or other (no-op if is obstacle)
		removeEntity := func(e *world.Entity, reason string) {
			data := e.Data()

			if data.Kind == world.EntityKindObstacle {
				return // obstacles never die
			}

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
		case boat != nil && weapon != nil:
			boat.Damage += weapon.Data().Damage

			if boat.Dead() {
				weapon.Owner.Score += 10 + boat.Owner.Score/4
				removeEntity(boat, fmt.Sprintf("Sunk by %s with a %s!", weapon.Owner.Name, weapon.Data().SubKind.Label()))
			}

			removeEntity(weapon, "hit")
		case boat != nil && otherBoat != nil:
			/*
				Goals:
				- (Canceled) At least one boat is guaranteed to receive fatal damage
				- Ships with near equal max health and near equal health
				  percentage both die (no seemingly arbitrary survivor) hence 110%
				- Low health boats still do damage, hence scale health percent
			*/
			damage := timeDeltaSeconds * 1.1 * min((boat.HealthPercent()*0.5+0.5)*boat.MaxHealth(), (otherBoat.HealthPercent()*0.5+0.5)*otherBoat.MaxHealth())

			posDiff := boat.Position.Sub(otherBoat.Position).Norm()

			boat.Damage += damage
			boat.Velocity += 3 * posDiff.Dot(boat.Direction.Vec2f())
			otherBoat.Damage += damage
			otherBoat.Velocity -= 3 * posDiff.Dot(otherBoat.Direction.Vec2f())

			if boat.Dead() {
				removeEntity(boat, fmt.Sprintf("Crashed into %s!", otherBoat.Owner.Name))
			}
			if otherBoat.Dead() {
				removeEntity(otherBoat, fmt.Sprintf("Crashed into %s!", boat.Owner.Name))
			}
		default:
			// Other ex weapon against weapon collision
			// note: will not remove obstacle entities
			removeEntity(entity, fmt.Sprintf("Crashed into %s!", other.Data().Label))
			removeEntity(other, fmt.Sprintf("Crashed into %s!", entity.Data().Label))
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

	// Loot is based on the length of the boat (TODO: with a small random factor)
	loot := int(data.Length * 0.25)
	for i := 0; i < loot; i++ {
		crate := &world.Entity{
			EntityType: world.EntityTypeCrate,
			Transform:  e.Transform,
		}

		h.spawnEntity(crate, data.Radius*0.5)
	}
}
