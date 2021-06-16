// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

import (
	"github.com/SoftbearStudios/mk48/server/world"
	"github.com/chewxy/math32"
	"math/rand"
	"runtime"
	"sync"
	"sync/atomic"
	"time"
)

const (
	// barrelRadius is the radius around an oil platform that barrels are counted.
	barrelRadius = 125
	// max amount of barrels around an oil platform
	platformBarrelCount = 12
	// platformBarrelSpawnRate is average seconds per barrel spawn.
	// Cant be less than spawnPeriod.
	platformBarrelSpawnRate = time.Second * 3
	// platformBarrelSpawnProb is the probably that a barrel will spawn around an oil platform.
	platformBarrelSpawnProb = float64(spawnPeriod) / float64(platformBarrelSpawnRate)
	// hq is this many times better than platform
	hqFactor = 2
)

// Spawn spawns non-boat/weapon entities such as collectibles and obstacles.
func (h *Hub) Spawn() {
	defer h.timeFunction("spawn", time.Now())

	// Outputs platforms that should spawn 1 barrel
	barrelSpawnerOutput := make(chan world.Vec2f, runtime.NumCPU()*2)
	barrelSpawnerPositions := make([]world.Vec2f, 0, 16)
	var wait sync.WaitGroup
	wait.Add(1)
	go func() {
		for position := range barrelSpawnerOutput {
			barrelSpawnerPositions = append(barrelSpawnerPositions, position)
		}
		wait.Done()
	}()

	// Use int64s for atomic ops
	currentCrateCount := int64(0)
	currentBarrelSpawnerCount := int64(0)

	h.world.SetParallel(true)
	h.world.ForEntities(func(entity *world.Entity) (stop, remove bool) {
		switch entity.Data().Kind {
		case world.EntityKindCollectible:
			atomic.AddInt64(&currentCrateCount, 1)
		case world.EntityKindObstacle:
			maxBarrels := 0
			spawnProb := 0.0
			switch entity.EntityType {
			case world.EntityTypeHQ:
				maxBarrels = platformBarrelCount * hqFactor
				spawnProb = platformBarrelSpawnProb * hqFactor
			case world.EntityTypeOilPlatform:
				maxBarrels = platformBarrelCount
				spawnProb = platformBarrelSpawnProb
			}
			if maxBarrels > 0 {
				if rand.Float64() < spawnProb {
					pos := entity.Position
					barrelCount := 0

					// Count current barrels
					h.world.ForEntitiesInRadius(pos, barrelRadius, func(_ float32, entity *world.Entity) (_ bool) {
						barrelCount++
						return
					})

					if barrelCount < maxBarrels {
						barrelSpawnerOutput <- pos
					}
				}

				atomic.AddInt64(&currentBarrelSpawnerCount, 1)
			}
		}
		return
	})
	h.world.SetParallel(false)

	close(barrelSpawnerOutput)
	wait.Wait()

	// Spawn barrels
	for _, data := range barrelSpawnerPositions {
		barrelEntity := &world.Entity{
			Transform: world.Transform{
				Position:  data,
				Velocity:  world.ToVelocity(rand.Float32()*10 + 10),
				Direction: world.ToAngle(rand.Float32() * math32.Pi * 2),
			},
			EntityType: world.EntityTypeBarrel,
		}
		h.spawnEntity(barrelEntity, barrelRadius*0.9)
	}

	// Spawn crates
	// Not all at once because then they decay all at once
	targetCollectibleCount := world.CrateCountOf(h.clients.Len)
	if maxCount := int(currentCrateCount) + 5 + targetCollectibleCount/60; targetCollectibleCount > maxCount {
		targetCollectibleCount = maxCount
	}

	for i := int(currentCrateCount); i < targetCollectibleCount; i++ {
		h.spawnEntity(&world.Entity{EntityType: world.EntityTypeCrate}, h.worldRadius)
	}

	// Spawn oil platforms
	targetObstacleCount := world.ObstacleCountOf(h.clients.Len)
	for i := int(currentBarrelSpawnerCount); i < targetObstacleCount; i++ {
		entity := &world.Entity{EntityType: world.EntityTypeOilPlatform}
		h.spawnEntity(entity, h.worldRadius)
	}
}

// spawnEntity spawns an entity and sets its owners EntityID if applicable.
// Returns if non zero EntityID if spawned.
// TODO fix this mess
func (h *Hub) spawnEntity(entity *world.Entity, initialRadius float32) world.EntityID {
	if initialRadius > 0 {
		radius := max(initialRadius, 1)
		center := entity.Position
		threshold := float32(5.0)

		governor := 0

		// Always randomize on first iteration
		for entity.Position == center || !h.canSpawn(entity, threshold) {
			// Pick a new position
			position := world.RandomAngle().Vec2f().Mul(math32.Sqrt(rand.Float32()) * radius)
			entity.Position = center.Add(position)
			entity.Direction = world.RandomAngle()

			radius = min(radius*1.1, h.worldRadius*0.9)
			threshold = 0.15 + threshold*0.85 // Approaches 1.0

			governor++
			if governor > 128 {
				// Don't take down the server just because cannnot
				// spawn an entity
				break
			}
		}

		entity.DirectionTarget = entity.Direction
	}

	if !h.canSpawn(entity, 1) {
		return world.EntityIDInvalid
	}

	// Outside world
	if entity.Position.LengthSquared() > h.worldRadius*h.worldRadius {
		return world.EntityIDInvalid
	}

	h.world.AddEntity(entity)
	entityID := entity.EntityID
	if entity.Owner != nil && entity.Data().Kind == world.EntityKindBoat {
		if entity.Owner.EntityID != world.EntityIDInvalid {
			panic("owner already has EntityID")
		}
		if entity.Owner.Respawning() {
			entity.Owner.ClearRespawn()
		}
		entity.Owner.EntityID = entityID
	}
	return entityID
}

// nearAny Returns if any entities are within a threshold for spawning (or if colliding with terrain)
func (h *Hub) canSpawn(entity *world.Entity, threshold float32) bool {
	switch entity.Data().Kind {
	case world.EntityKindCollectible, world.EntityKindDecoy, world.EntityKindWeapon:
		// Weapons spawn where the player shoots them regardless of entities,
		// Collectibles don't care about colliding with entities while spawning

		// Simply perform a terrain check against the current position (no slow conservative check)
		return !h.terrain.Collides(entity, 0)
	case world.EntityKindBoat:
		// Be picky about spawning in appropriate depth water
		// unless threshold is very low
		// Ignore if owner has a team or enough points to upgrade to bigger ship
		if threshold > 1.5 && (entity.Owner == nil || (entity.Owner.TeamID == world.TeamIDInvalid && entity.Owner.Score < world.LevelToScore(2))) {
			belowKeel := entity.BelowKeel(h.terrain)
			if belowKeel < 0 || belowKeel > 5 {
				return false
			}
		}
	}

	// Slow, conservative check
	if h.terrain.Collides(entity, -1) {
		return false
	}

	// Extra space between entities
	radius := entity.Data().Radius
	maxT := (radius + world.EntityRadiusMax) * threshold

	return !h.world.ForEntitiesInRadius(entity.Position, maxT, func(r float32, otherEntity *world.Entity) (stop bool) {
		t := (radius + otherEntity.Data().Radius) * threshold
		return r < t*t
	})
}
