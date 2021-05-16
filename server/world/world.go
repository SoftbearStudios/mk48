// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"fmt"
	"github.com/chewxy/math32"
	"math/rand"
	"testing"
)

const (
	MinRadius = 500

	// PlayerSpace Target space (square meters) per each
	PlayerSpace   = 300000
	CrateSpace    = 30000
	ObstacleSpace = 1000000
)

// World A world holds entities
type World interface {
	// AddEntity Adds a new entity to its sector
	// Cannot hold pointer after call is finished
	AddEntity(entity *Entity) EntityID

	// Count returns number of entities in the world
	// Cannot be called concurrently with writes
	Count() int

	// Debug Prints debug output to os.Stdout
	// Cannot be called concurrently with writes
	Debug()

	// EntityByID Gets an entity by its id
	// For reading and writing
	// Cannot hold pointers after call is finished
	EntityByID(entityID EntityID, callback func(entity *Entity) (remove bool))

	// ForEntities Iterates all the entities and returns if stopped early
	// For reading and writing
	// Cannot hold pointer after call is finished
	ForEntities(callback func(entityID EntityID, entity *Entity) (stop, remove bool)) bool

	// ForEntitiesInRadius Iterates all the entities in a radius and returns if stopped early
	// Only for reading so no adding or modifying entities
	// Cannot hold pointer after call is finished
	ForEntitiesInRadius(position Vec2f, radius float32, callback func(r float32, entityID EntityID, entity *Entity) (stop bool)) bool

	// ForEntitiesAndOthers Iterates all the entities and other entities in a radius and returns if stopped early
	// For reading and writing
	// Cannot hold pointers after call is finished
	// Cannot modify position of entities
	// If entities are added during iteration may not iterate them
	// Skips radii that are <= 0
	// Radii that are too big cause all sectors to be iterated
	ForEntitiesAndOthers(entityCallback func(entityID EntityID, entity *Entity) (stop bool, radius float32),
		otherCallback func(entityID EntityID, entity *Entity, otherEntityID EntityID, otherEntity *Entity) (stop, remove, removeOther bool)) bool

	// Resize sets the max size of the World.
	// It may or may not reallocate parts of the World depending on the difference in radius.
	Resize(radius float32)

	// SetParallel Marks world as ready only for concurrent reads
	// Cannot remove or add during read only mode
	// Returns if can be read concurrently
	SetParallel(parallel bool) bool
}

func AreaOf(playerCount int) float32 {
	return float32(playerCount * PlayerSpace)
}

func RadiusOf(playerCount int) float32 {
	area := AreaOf(playerCount)
	radius := math32.Sqrt(area / math32.Pi)
	return max(MinRadius, radius)
}

func CrateCountOf(playerCount int) int {
	return int(AreaOf(playerCount) / CrateSpace)
}

func ObstacleCountOf(playerCount int) int {
	return int(AreaOf(playerCount) / ObstacleSpace)
}

type testWorld struct {
	world     World
	entityIDs []EntityID
	radius    int
}

func createTestWorlds(create func(radius int) World, end int) []testWorld {
	var testWorlds []testWorld
	radius := 500
	for i := 64; i <= end; i *= 4 {
		world := createTestWorld(create(radius), i, radius)
		// world.world.Debug()
		testWorlds = append(testWorlds, world)
		radius *= 2
	}
	return testWorlds
}

func Test(t *testing.T, create func(radius int) World) {
	// TODO implement
}

func Bench(b *testing.B, create func(radius int) World, end int) {
	testWorlds := createTestWorlds(create, end)

	for _, w := range testWorlds {
		world := w
		b.Run(fmt.Sprintf("EntityByID/%d", len(world.entityIDs)), func(b *testing.B) {
			_ = testWorldEntityByID(world, b.N)
		})
	}

	for _, w := range testWorlds {
		world := w
		b.Run(fmt.Sprintf("InRadius/%d", len(world.entityIDs)), func(b *testing.B) {
			_ = testWorldInRadius(world, b.N)
		})
	}

	for _, w := range testWorlds {
		world := w
		b.Run(fmt.Sprintf("Iterate/%d", len(world.entityIDs)), func(b *testing.B) {
			_ = testWorldIterate(world, b.N)
		})
	}

	for _, w := range testWorlds {
		world := w
		if world.world.SetParallel(true) {
			b.Run(fmt.Sprintf("IterateParallel/%d", len(world.entityIDs)), func(b *testing.B) {
				testWorldIterateParallel(world, b.N)
			})
			world.world.SetParallel(false)
		}
	}

	for _, w := range testWorlds {
		world := w
		b.Run(fmt.Sprintf("IterateRadius/%d", len(world.entityIDs)), func(b *testing.B) {
			_ = testWorldIterateRadius(world, b.N)
		})
	}
}

func createTestWorld(world World, entityCount int, radius int) testWorld {
	entityIDs := make([]EntityID, entityCount)

	floatRadius := float32(radius)

	for i := 0; i < entityCount; i++ {
		entityType := EntityType(rand.Intn(EntityTypeCount-1) + 1)
		pos := Vec2f{X: rand.Float32()*floatRadius*2 - floatRadius, Y: rand.Float32()*floatRadius*2 - floatRadius}

		entity := Entity{
			EntityType: entityType,
			Transform: Transform{
				Position:  pos,
				Velocity:  rand.Float32() * entityType.Data().Speed,
				Direction: Angle(rand.Float32() * math32.Pi * 2),
			},
		}
		entityIDs[i] = world.AddEntity(&entity)
	}

	return testWorld{
		world:     world,
		entityIDs: entityIDs,
		radius:    radius,
	}
}

func testWorldEntityByID(world testWorld, times int) int {
	entityTypeCounts := make([]int, EntityTypeCount)

	j := 0
	for i := 0; i < times; i++ {
		entityID := world.entityIDs[j]
		world.world.EntityByID(entityID, func(entity *Entity) (_ bool) {
			entityTypeCounts[int(entity.EntityType)]++
			return
		})
		j++
		if j == len(world.entityIDs) {
			j = 0
		}
	}

	total := 0
	for _, c := range entityTypeCounts {
		total += c
	}
	return total
}

func testWorldInRadius(world testWorld, times int) int {
	entityTypeCounts := make([]int, EntityTypeCount)

	// Only allocate callbacks once
	var entity *Entity
	i := 0

	callback2 := func(_ float32, _ EntityID, otherEntity *Entity) (_ bool) {
		entityTypeCounts[int(entity.EntityType)]++
		entityTypeCounts[int(otherEntity.EntityType)]++
		return
	}

	callback1 := func(_ EntityID, e *Entity) (stop, _ bool) {
		i++
		if stop = i >= times; stop {
			return
		}

		entity = e
		radius := entity.Data().Radius * 2

		world.world.ForEntitiesInRadius(entity.Position, radius, callback2)

		return
	}

	for !world.world.ForEntities(callback1) {

	}

	total := 0
	for _, c := range entityTypeCounts {
		total += c
	}
	return total
}

func testWorldIterate(world testWorld, times int) int {
	entityTypeCounts := make([]int, EntityTypeCount)

	for i := 0; i < times; {
		world.world.ForEntities(func(entityID EntityID, entity *Entity) (stop, _ bool) {
			entityTypeCounts[int(entity.EntityType)]++
			i++
			stop = i >= times

			return
		})
	}

	total := 0
	for _, c := range entityTypeCounts {
		total += c
	}
	return total
}

func testWorldIterateParallel(world testWorld, times int) int {
	//entityTypeCounts := make([]struct {
	//	count int64
	//	_     [7]uint64
	//}, EntityTypeCount)
	count := world.world.Count()

	for i := 0; i < times; i += count {
		world.world.ForEntities(func(entityID EntityID, entity *Entity) (_, _ bool) {
			entityType := entity.EntityType
			if entityType == 0 {
				panic("invalid entity type")
			}
			// Don't count for now because of false sharing
			// atomic.AddInt64(&entityTypeCounts[int(entity.EntityType)].count, 1)
			return
		})
	}

	total := 0
	//for _, c := range entityTypeCounts {
	//	total += int(c.count)
	//}
	return total
}

func testWorldIterateRadius(world testWorld, times int) int {
	entityTypeCounts := make([]int, EntityTypeCount)

	for i := 0; i < times; {
		world.world.ForEntitiesAndOthers(func(entityID EntityID, entity *Entity) (stop bool, radius float32) {
			i++
			stop = i >= times
			radius = entity.Data().Radius * 2

			return
		}, func(entityID EntityID, entity *Entity, otherEntityID EntityID, otherEntity *Entity) (_, _, _ bool) {
			entityTypeCounts[int(entity.EntityType)]++
			entityTypeCounts[int(otherEntity.EntityType)]++
			return
		})
	}

	total := 0
	for _, c := range entityTypeCounts {
		total += c
	}
	return total
}
