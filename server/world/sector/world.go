// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package sector

import (
	"fmt"
	"github.com/chewxy/math32"
	"math"
	"mk48/server/world"
)

const (
	size         = 500       // Meters
	maxLength    = 1<<15 - 1 // size units
	minSectorCap = 4         // Capacity to start sectors with
)

// bufferSectorIndex is a sentinel value to mark that entity is in buffer.
var bufferSectorIndex = sectorIndex{sectorID: sectorID{x: math.MinInt16, y: math.MinInt16}, index: -1}

type (
	// World is an implementation of world.World which divides entities into sectors
	World struct {
		sectors     []sector                       // sectors stores the entities in spatial partitions
		buffered    []sectorEntity                 // buffered is entities added during a read
		entityIDs   map[world.EntityID]sectorIndex // entityIDs stores where to find the entities
		entityCount int                            // cached number of entities
		width       uint16                         // width is cross section in sector space
		logWidth    uint8                          // logWidth is log2(width)
		depth       int8                           // call depth
		parallel    bool                           // no writing during parallel
	}

	// sector is one bucket of the World
	sector struct {
		entities []sectorEntity
	}

	// sectorEntity stores its id as opposed to it being stored in map[world.EntityID]*world.Entity
	sectorEntity struct {
		world.Entity
		world.EntityID
	}
)

// New creates a new World.
func New(radius float32) *World {
	w := &World{
		entityIDs: make(map[world.EntityID]sectorIndex),
		buffered:  make([]sectorEntity, 0, 16),
	}

	// Resize allocates World.sectors
	w.Resize(radius)

	return w
}

func (w *World) Count() int {
	return w.entityCount
}

// AddEntity adds an entity to the world
// Cannot add during parallel execution
func (w *World) AddEntity(entity *world.Entity) world.EntityID {
	if w.parallel {
		panic("cannot write")
	}
	e := &sectorEntity{Entity: *entity, EntityID: world.AllocateEntityID(func(id world.EntityID) bool {
		_, ok := w.entityIDs[id]
		return ok
	})}
	w.entityCount++

	if w.depth > 0 {
		// Mark used so don't reuse entityID in buffer
		w.entityIDs[e.EntityID] = bufferSectorIndex
		w.buffered = append(w.buffered, *e)
	} else {
		w.setEntity(e)
	}
	return e.EntityID
}

// Debug output
func (w *World) Debug() {
	fmt.Printf("sector world: sectors: %d, entities: %d \n", len(w.sectors), w.Count())
}

// EntityByID gets an entity by its id
// Cannot hold references to entity outside this function
func (w *World) EntityByID(entityID world.EntityID, callback func(entity *world.Entity) (remove bool)) {
	fullID, ok := w.entityIDs[entityID]
	if !ok {
		callback(nil)
		return
	}

	s := w.sector(fullID.sectorID)

	w.addDepth(1)
	remove := callback(&s.entities[fullID.index].Entity)
	w.addDepth(-1)

	if remove {
		if w.depth != 0 || w.parallel {
			panic("cannot write")
		}
		w.remove(fullID.sectorID, s, int(fullID.index), false)
	}

	if w.depth == 0 && len(w.buffered) > 0 {
		w.addBuffered()
	}

	return
}

func (w *World) Resize(radius float32) {
	w.assertDepth(0)

	intWidth := int(radius*(1.0/size))*2 + 1
	if radius < 0 || intWidth > math32.MaxInt16/2 {
		panic("radius out of range")
	}

	width := uint16(intWidth)
	if width <= w.width {
		// No resize necessary
		return
	}
	width = nextPowerOf2(width)

	sectors := make([]sector, int(width)*int(width))
	oldSectors := w.sectors

	oldWidth := w.width
	oldLogWidth := w.logWidth

	for i, s := range oldSectors {
		if len(s.entities) == 0 {
			continue
		}
		sectors[sliceIndexSectorID(i, oldWidth, oldLogWidth).sliceIndex(width)] = s
	}

	w.sectors = sectors
	w.width = width
	w.logWidth = log2(width)
}

// SetParallel turns on parallel execution mode
func (w *World) SetParallel(parallel bool) bool {
	w.assertDepth(0)
	w.parallel = parallel
	return true
}

// addBuffered adds buffered entities
func (w *World) addBuffered() {
	for i := range w.buffered {
		// Doesn't keep alive pointer
		w.setEntity(&w.buffered[i])

		// Clear pointer
		w.buffered[i] = sectorEntity{}
	}

	// Clear for next use
	w.buffered = w.buffered[:0]
}

// addDepth increases the World function call depth so AddEntity adds to buffered
func (w *World) addDepth(depth int8) {
	if !w.parallel {
		w.depth += depth
	}
}

// assertDepth tests the World function call depth for debugging
func (w *World) assertDepth(depth int8) {
	if w.depth != depth {
		panic(fmt.Sprintf("invalid iteration depth %d want %d", w.depth, depth))
	}
}

func (w *World) sector(id sectorID) *sector {
	index := id.sliceIndex(w.width)
	if index == -1 {
		return nil
	}
	return &w.sectors[index]
}

// remove removes an entity from a sector given its index and returns new index for loops
// Cannot call if any *Entity are in use because it moves around elements of the slice
// if move entity isn't closed and is instead added to a different sector
func (w *World) remove(id sectorID, s *sector, index int, move bool) int {
	entity := &s.entities[index]
	if move {
		// Put the entity where it belongs
		w.setEntity(entity)
	} else {
		// Delete the entity
		w.entityCount--
		entity.Close()
		delete(w.entityIDs, entity.EntityID)
	}
	// Cannot use entity past this point

	end := len(s.entities) - 1
	// Move other's sectorIndex pointer
	if index != end {
		s.entities[index] = s.entities[end]
		w.entityIDs[s.entities[index].EntityID] = sectorIndex{sectorID: id, index: int32(index)}
	}

	// Clear pointer
	s.entities[end] = sectorEntity{}
	s.entities = s.entities[:end]

	if len(s.entities) == 0 {
		// Delete slice if no more entities
		w.sectors[id.sliceIndex(w.width)].entities = nil
	} else if c := cap(s.entities) / 2; len(s.entities)+minSectorCap/2 < c {
		// Shrink to use less memory
		entities := make([]sectorEntity, len(s.entities), c)
		copy(entities, s.entities)
		s.entities = entities
	}

	return index - 1
}

// setEntity adds an existing entity to its sector and changes its sectorIndex pointer
func (w *World) setEntity(e *sectorEntity) {
	id := vec2fSectorID(e.Position)
	s := w.sector(id)

	i := len(s.entities)
	s.entities = append(s.entities, *e)

	// Set sectorIndex pointer
	w.entityIDs[e.EntityID] = sectorIndex{sectorID: id, index: int32(i)}
}
