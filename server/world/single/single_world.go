// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package single

import (
	"fmt"
	"github.com/SoftbearStudios/mk48/server/world"
)

// A world holds entities
type World struct {
	entities map[world.EntityID]*world.Entity
}

func New() *World {
	return &World{
		entities: make(map[world.EntityID]*world.Entity),
	}
}

func (w *World) Count() int {
	return len(w.entities)
}

func (w *World) AddEntity(entity *world.Entity) {
	entity.EntityID = world.AllocateEntityID(func(id world.EntityID) bool {
		_, ok := w.entities[id]
		return ok
	})
	w.entities[entity.EntityID] = entity
}

func (w *World) EntityByID(entityID world.EntityID, callback func(entity *world.Entity) (remove bool)) {
	entity := w.entities[entityID]
	if callback(entity) && entity != nil {
		w.removeEntity(entityID, entity)
	}
}

func (w *World) ForEntities(callback func(entity *world.Entity) (stop, remove bool)) bool {
	for entityID, entity := range w.entities {
		stop, remove := callback(entity)
		if remove {
			w.removeEntity(entityID, entity)
		}
		if stop {
			return true
		}
	}
	return false
}

func (w *World) ForEntitiesInRadius(position world.Vec2f, radius float32, callback func(radius float32, entity *world.Entity) (stop bool)) bool {
	r2 := radius * radius
	for _, entity := range w.entities {
		r := position.DistanceSquared(entity.Position)
		if r > r2 {
			continue
		}
		if callback(r, entity) {
			return true
		}
	}
	return false
}

func (w *World) ForEntitiesAndOthers(entityCallback func(entity *world.Entity) (stop bool, radius float32),
	otherCallback func(entityID world.EntityID, entity *world.Entity, otherEntityID world.EntityID, otherEntity *world.Entity) (stop, remove, removeOther bool)) bool {

	for entityID, entity := range w.entities {
		stop, radius := entityCallback(entity)
		if stop {
			return true
		}
		r2 := radius * radius
		for otherID, other := range w.entities {
			if entity.Position.DistanceSquared(other.Position) > r2 {
				continue
			}

			stopInner, remove, removeOther := otherCallback(entityID, entity, otherID, other)

			if remove {
				w.removeEntity(entityID, entity)
			}

			if removeOther {
				w.removeEntity(otherID, other)
			}

			// Stop early if entity is removed
			if stopInner || remove {
				stop = stopInner
				break
			}
		}
	}
	return false
}

// Ignore for now
func (w *World) SetParallel(_ bool) bool {
	return true
}

func (w *World) Debug() {
	fmt.Printf("single world: entities: %d\n", w.Count())
}

func (w *World) Resize(radius float32) {
	// Do nothing
}

func (w *World) removeEntity(entityID world.EntityID, entity *world.Entity) {
	entity.Close()
	delete(w.entities, entityID)
}
