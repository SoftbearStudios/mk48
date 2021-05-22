// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package sector

import (
	"mk48/server/world"
	"runtime"
	"sync/atomic"
	"unsafe"
)

func (w *World) ForEntities(callback func(entity *world.Entity) (stop, remove bool)) bool {
	if cpus := runtime.NumCPU(); cpus > 1 && w.parallel {
		return w.forEntitiesParallel(callback, cpus)
	}

	canWrite := w.depth == 0
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
		for i := 0; i < len(s.entities); i++ {
			entity := &s.entities[i]
			oldPos := entity.Position

			stop, remove := callback(entity)

			var move bool
			if entity.Position != oldPos {
				if newSectorID := vec2fSectorID(entity.Position); id != newSectorID {
					move = !remove
					remove = true
				}
			}

			if remove {
				if !canWrite {
					panic("cannot write")
				}
				i = w.remove(id, s, i, move)
			}

			if canWrite && len(w.buffered) > 0 {
				w.addBuffered()
			}

			if stop {
				w.addDepth(-1)
				return true
			}
		}
	}

	w.addDepth(-1)
	return false
}

func (w *World) forEntitiesParallel(callback func(entity *world.Entity) (stop, remove bool), cpus int) bool {
	type removal struct {
		world.EntityID
		move bool
	}

	finished := int64(cpus)
	sliceIndex := int64(0)
	output := make(chan removal, cpus)

	// The workers close when sliceIndex >= len(w.sectors)
	for c := 0; c < cpus; c++ {
		go func(index *int64, out chan<- removal, f *int64) {
			width := w.width
			logWidth := w.logWidth
			sectors := w.sectors

			for {
				// Process sectorsPerAdd items at a time
				const sectorsPerAdd = 8

				end := int(atomic.AddInt64(index, sectorsPerAdd))
				start := end - sectorsPerAdd

				if start < 0 {
					start = 0
				}

				if end > len(sectors) {
					end = len(sectors)

					if start >= len(sectors) {
						// No more sectors left so last to exit closes the output channel
						if atomic.AddInt64(&finished, -1) == 0 {
							close(output)
						}
						return
					}
				}

				// Bounds check elimination doesn't work
				// _ = sectors[start:end]

				for ; start < end; start++ {
					entities := sectors[start].entities

					for i := range entities {
						entity := &entities[i]
						// alias Vec2f as a uint64 to avoid FP instructions ~10% faster
						oldPos := *(*uint64)(unsafe.Pointer(&entity.Position))

						stop, remove := callback(entity)
						if stop {
							panic("cannot stop during parallel")
						}

						var move bool
						if *(*uint64)(unsafe.Pointer(&entity.Position)) == oldPos {
							// Defaults branch to taken
						} else {
							id := sliceIndexSectorID(start, width, logWidth)
							if newSectorID := vec2fSectorID(entity.Position); id != newSectorID {
								move = !remove
								remove = true
							}
						}

						if remove {
							// Removals modify other sectors / world and can only can do removals on single thread
							out <- removal{EntityID: entity.EntityID, move: move}
						}
					}
				}
			}
		}(&sliceIndex, output, &finished)
	}

	// Collect all removals until last worker exits
	removals := make([]removal, 0, 64)
	for r := range output {
		removals = append(removals, r)
	}

	// Single threaded remover
	for _, r := range removals {
		entityID := r.EntityID
		move := r.move

		sectorLocation, ok := w.entityIDs[entityID]
		if !ok {
			panic("sectorIndex not found")
		}

		id := sectorLocation.sectorID
		s := w.sector(id)

		w.remove(id, s, int(sectorLocation.index), move)
	}

	return false
}
