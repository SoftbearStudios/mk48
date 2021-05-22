// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"github.com/chewxy/math32"
	"math/rand"
	"mk48/server/world"
	"runtime"
	"sync"
	"sync/atomic"
	"time"
)

const (
	// barrelRadius is the radius around an oil platform that barrels are counted.
	barrelRadius = 125
	// targetBarrelCount is max amount of barrels around an oil platform.
	targetBarrelCount = 20
	// barrelSpawnRate is average seconds per barrel spawn.
	// Cant be less than spawnPeriod.
	barrelSpawnRate = time.Second * 2
	// barrelSpawnProb is the probably that a barrel will spawn around an oil platform.
	barrelSpawnProb = float64(spawnPeriod) / float64(barrelSpawnRate)
)

// Spawn spawns non-boat/weapon entities such as collectibles and obstacles.
func (h *Hub) Spawn() {
	defer h.timeFunction("spawn", time.Now())

	// Outputs platforms that should spawn 1 barrel
	platformOutput := make(chan world.Vec2f, runtime.NumCPU()*2)
	platformPositions := make([]world.Vec2f, 0, 16)
	var wait sync.WaitGroup
	wait.Add(1)
	go func() {
		for position := range platformOutput {
			platformPositions = append(platformPositions, position)
		}
		wait.Done()
	}()

	// Use int64s for atomic ops
	currentCrateCount := int64(0)
	currentOilPlatformCount := int64(0)

	h.world.SetParallel(true)
	h.world.ForEntities(func(entity *world.Entity) (stop, remove bool) {
		switch entity.Data().Kind {
		case world.EntityKindCollectible:
			atomic.AddInt64(&currentCrateCount, 1)
		case world.EntityKindObstacle:
			if entity.EntityType == world.EntityTypeOilPlatform && rand.Float64() < barrelSpawnProb {
				pos := entity.Position
				barrelCount := 0

				// Count current barrels
				h.world.ForEntitiesInRadius(pos, barrelRadius, func(_ float32, entity *world.Entity) (_ bool) {
					barrelCount++
					return
				})

				if barrelCount < targetBarrelCount {
					platformOutput <- pos
				}
			}
			atomic.AddInt64(&currentOilPlatformCount, 1)
		}
		return
	})
	h.world.SetParallel(false)

	close(platformOutput)
	wait.Wait()

	// Spawn barrels
	for _, data := range platformPositions {
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
	for i := int(currentOilPlatformCount); i < targetObstacleCount; i++ {
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

		// Always randomize on first iteration
		for entity.Position == center || (entity.Data().Kind != world.EntityKindCollectible &&
			entity.Data().Kind != world.EntityKindWeapon && h.nearAny(entity, threshold)) {

			angle := world.ToAngle(rand.Float32() * 2 * math32.Pi)
			position := angle.Vec2f().Mul(math32.Sqrt(rand.Float32()) * radius)
			entity.Position = center.Add(position)

			angle = world.ToAngle(rand.Float32() * 2 * math32.Pi)
			entity.Direction = angle

			radius = min(radius*1.1, h.worldRadius)
			threshold = 0.25 + threshold*0.75 // Approaches 1.0
		}

		entity.DirectionTarget = entity.Direction
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

// nearAny Returns if any entities are within a threshold for spawning.
func (h *Hub) nearAny(entity *world.Entity, threshold float32) bool {
	// Extra space between entities

	radius := entity.Data().Radius
	maxT := (radius + world.EntityRadiusMax) * threshold

	return h.terrain.Collides(entity, 1) || h.world.ForEntitiesInRadius(entity.Position, maxT, func(r float32, otherEntity *world.Entity) (stop bool) {
		t := (radius + otherEntity.Data().Radius) * threshold
		return r < t*t
	})
}
