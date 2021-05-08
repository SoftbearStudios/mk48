// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package sector

import "mk48/server/world"

// ForEntitiesAndOthers TODO support multi-threading
func (w *World) ForEntitiesAndOthers(entityCallback func(entityID world.EntityID, entity *world.Entity) (stop bool, radius float32),
	otherCallback func(entityID world.EntityID, entity *world.Entity, otherEntityID world.EntityID, otherEntity *world.Entity) (stop, remove, removeOther bool)) bool {

	canWrite := w.depth == 0 && !w.parallel
	w.addDepth(1)

	width := w.width
	logWidth := w.logWidth
	sectors := w.sectors

	for i := range sectors {
		s := &sectors[i]
		if len(s.entities) == 0 {
			continue
		}

		id := sliceIndexSectorID(i, width, logWidth)
		for i := 0; i < len(s.entities); {
			e := &s.entities[i]

			// Position must not be modified
			stopSector, radius := entityCallback(e.EntityID, &e.Entity)

			if canWrite && len(w.buffered) > 0 {
				w.addBuffered()
			}

			if stopSector {
				w.addDepth(-1)
				return true
			}

			nextI := i + 1 // If continue loop, just set i = nextI

			if radius <= 0.0 {
				i = nextI
				continue
			}

			r2 := radius * radius

			// 'i' can change if entities are removed so lookup with 'i' each time to get entity
			w.forSectorsInRadius(s.entities[i].Position, radius, func(otherSectorID sectorID, otherSector *sector) (stop bool) {
				for j := 0; j < len(otherSector.entities); j++ {
					entity := &s.entities[i]
					other := &otherSector.entities[j]

					// Out of radius or same entity
					if entity.Position.DistanceSquared(other.Position) > r2 || entity == other {
						continue
					}

					var remove, removeOther bool
					// Position must not be modified
					stop, remove, removeOther = otherCallback(entity.EntityID, &entity.Entity, other.EntityID, &other.Entity)

					if removeOther {
						if !canWrite {
							panic("cannot write")
						}
						if end := len(otherSector.entities) - 1; otherSector == s && i == end {
							// This just acknowledges the subsequent w.remove's effect on i
							i = j
						}
						j = w.remove(otherSectorID, otherSector, j, false)
					}

					if remove {
						if !canWrite {
							panic("cannot write")
						}
						nextI--
						i = w.remove(id, s, i, false)
					}

					if canWrite && len(w.buffered) > 0 {
						w.addBuffered()
					}

					// Stop nested iteration if entity is removed but don't stop top level iteration
					if remove || stop {
						stopSector = stop
						stop = remove

						break
					}
				}
				return
			})

			if stopSector {
				w.addDepth(-1)
				return true
			}

			i = nextI
		}
	}

	w.addDepth(-1)
	return false
}
