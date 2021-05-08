// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package sector

import (
	"mk48/server/world"
)

// Iterates all the sectors in a radius and returns if stopped early
// If sectors are added during iteration may not iterate them
// currentSector is the sector that the position is in if it is known
func (w *World) forSectorsInRadius(position world.Vec2f, radius float32, callback func(sectorID sectorID, sector *sector) (stop bool)) bool {
	width := w.width
	min := -int16(width / 2)
	max := int16(width/2 - 1)

	minSectorID := vec2fSectorID(position.Sub(world.Vec2f{X: radius, Y: radius})).min(min)
	maxSectorID := vec2fSectorID(position.Add(world.Vec2f{X: radius, Y: radius})).max(max)

	width2 := int(width)
	sectors := w.sectors

	// Iterate y in outer for better locality of reference
	for y := minSectorID.y; y <= maxSectorID.y; y++ {
		for x := minSectorID.x; x <= maxSectorID.x; x++ {
			id := sectorID{x: x, y: y}
			if !id.inRadius(position, radius) {
				continue
			}

			s := &sectors[int(x-min)+int(y-min)*width2]
			if len(s.entities) == 0 {
				continue
			}

			if callback(id, s) {
				return true
			}
		}
	}

	return false
}

// ForEntitiesInRadius implements world.World.ForEntitiesInRadius
// For reading only
func (w *World) ForEntitiesInRadius(position world.Vec2f, radius float32, callback func(r float32, entityID world.EntityID, entity *world.Entity) (stop bool)) bool {
	w.addDepth(1)

	r2 := radius * radius
	stopped := w.forSectorsInRadius(position, radius, func(sectorID sectorID, s *sector) bool {
		// Store entities in local variable so compiler knows it doesn't change
		entities := s.entities
		for i := range entities {
			entity := &entities[i]
			entityPos := entity.Position

			r := position.DistanceSquared(entityPos)
			if r > r2 {
				continue
			}

			if callback(r, entity.EntityID, &entity.Entity) {
				return true
			}
		}
		return false
	})

	w.addDepth(-1)
	return stopped
}
