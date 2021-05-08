// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package compressed

import (
	"fmt"
	"math/rand"
	"mk48/server/terrain"
	"mk48/server/world"
	"sync"
	"sync/atomic"
	"time"
	"unsafe"
)

const Size = 2048

type Terrain struct {
	generator  terrain.Source
	chunks     [Size / chunkSize][Size / chunkSize]*chunk
	chunkCount int32
	mutex      sync.Mutex
}

func New(generator terrain.Source) *Terrain {
	return &Terrain{
		generator: generator,
	}
}

func (t *Terrain) clamp(aabb world.AABB) (aabb2 world.AABB, x, y, width, height int) {
	minX, minY := t.start()
	maxX, maxY := t.end()

	p := aabb.Vec2f.Mul(1.0 / terrain.Scale).Floor()
	x = maxInt(minX, int(p.X))
	y = maxInt(minY, int(p.Y))

	if x >= maxX || y >= maxY {
		return
	}

	s := world.Vec2f{X: aabb.Width, Y: aabb.Height}.Mul(1.0 / terrain.Scale).Ceil()
	width = minInt(maxX-x, int(s.X)+1)
	height = minInt(maxY-y, int(s.Y)+1)

	aabb2 = world.AABB{
		Vec2f: world.Vec2f{
			X: float32(x),
			Y: float32(y),
		}.Mul(terrain.Scale),
		Width:  float32(width-1) * terrain.Scale,
		Height: float32(height-1) * terrain.Scale,
	}

	return
}

func (t *Terrain) Clamp(aabb world.AABB) world.AABB {
	clamped, _, _, _, _ := t.clamp(aabb)
	return clamped
}

func (t *Terrain) At(aabb world.AABB) *terrain.Data {
	clamped, x, y, width, height := t.clamp(aabb)

	data := terrain.NewData()
	buffer := Buffer{
		buf: data.Data,
	}

	for j := 0; j < height; j++ {
		for i := 0; i < width; i++ {
			buffer.writeByte(t.at(x+i, y+j))
		}
	}

	data.AABB = clamped
	data.Data = buffer.Buffer()
	data.Stride = width
	data.Length = width * height

	return data
}

func (t *Terrain) Repair() {
	millis := time.Now().UnixNano() / int64(time.Millisecond/time.Nanosecond)
	for ucx, chunks := range t.chunks {
		for ucy, c := range chunks {
			if c != nil && millis >= c.regen {
				if c.regen != 0 { // Don't regen the first time
					generateChunk(t.generator, ucx-Size/chunkSize/2, ucy-Size/chunkSize/2, c)
				}
				c.regen = millis + regenMillis + int64(rand.Intn(10000)) // add some randomness to avoid simultaneous regen
			}
		}
	}
}

// Changes the terrain height at pos by change
func (t *Terrain) Sculpt(pos world.Vec2f, change float32) {
	pos = pos.Mul(1.0 / terrain.Scale)

	// Floor and Ceiling pos
	cPos := pos.Ceil()
	fPos := pos.Floor()

	delta := pos.Sub(fPos)

	// Set 4x4 grid
	// 00 10
	// 01 11
	t.set2(fPos, clampToGrassByte(float32(t.at2(fPos))+change*(2-delta.X-delta.Y)*0.5))
	t.set2(world.Vec2f{X: cPos.X, Y: fPos.Y}, clampToGrassByte(float32(t.at2(world.Vec2f{X: cPos.X, Y: fPos.Y}))*change*(1+delta.X-delta.Y)*0.5))
	t.set2(world.Vec2f{X: fPos.X, Y: cPos.Y}, clampToGrassByte(float32(t.at2(world.Vec2f{X: fPos.X, Y: cPos.Y}))*change*(1-delta.X+delta.Y)*0.5))
	t.set2(cPos, clampToGrassByte(float32(t.at2(cPos))+change*(delta.X+delta.Y)/2))
}

func (t *Terrain) Collides(entity *world.Entity, seconds float32) bool {
	data := entity.Data()

	if data.Radius < 4 {
		// Not worth doing multiple terrain samples for small entities
		return t.atPos(entity.Position) > terrain.OceanLevel-6
	}

	sweep := seconds * entity.Velocity

	const graceMargin = 0.9
	dimensions := world.Vec2f{X: data.Length + sweep, Y: data.Width}.Mul(0.5 * graceMargin)
	normal := entity.Direction.Vec2f()
	tangent := normal.Rot90()
	position := entity.Position.AddScaled(normal, sweep/2)

	for l := -dimensions.X; l < dimensions.X; l += min(16, dimensions.Y*0.333) {
		for w := -dimensions.Y; w < dimensions.Y; w += min(8, dimensions.Y*0.49) {
			if t.atPos(position.Add(normal.Mul(l)).Add(tangent.Mul(w))) > terrain.OceanLevel-6 {
				return true
			}
		}
	}

	return false
}

func (t *Terrain) Debug() {
	// Take lock so all generation is done
	t.mutex.Lock()
	defer t.mutex.Unlock()

	fmt.Println("compressed terrain: chunks:", t.chunkCount)
}

func (t *Terrain) atPos(pos world.Vec2f) byte {
	pos = pos.Mul(1.0 / terrain.Scale)

	// Floor and Ceiling pos
	cPos := pos.Ceil()
	fPos := pos.Floor()

	delta := pos.Sub(fPos)

	// Sample 4x4 grid
	// 00 10
	// 01 11
	c00 := t.at2(fPos)
	c10 := t.at2(world.Vec2f{X: cPos.X, Y: fPos.Y})
	c01 := t.at2(world.Vec2f{X: fPos.X, Y: cPos.Y})
	c11 := t.at2(cPos)

	return blerp(c00, c10, c01, c11, delta.X, delta.Y)
}

func (t *Terrain) at2(terrainPos world.Vec2f) byte {
	minX, minY := t.start()
	maxX, maxY := t.end()

	x, y := int(terrainPos.X), int(terrainPos.Y)
	if x >= minX && x < maxX && y >= minY && y < maxY {
		return t.at(x, y)
	}
	return 0
}

func (t *Terrain) set2(terrainPos world.Vec2f, value byte) {
	minX, minY := t.start()
	maxX, maxY := t.end()

	x, y := int(terrainPos.X), int(terrainPos.Y)
	if x >= minX && x < maxX && y >= minY && y < maxY {
		t.set(x, y, value)
	}
}

func (t *Terrain) at(x, y int) byte {
	x += Size / 2
	y += Size / 2

	c := t.getChunk(x, y)
	return c.at(uint(x&(chunkSize-1)), uint(y&(chunkSize-1)))
}

func (t *Terrain) set(x, y int, value byte) {
	x += Size / 2
	y += Size / 2

	c := t.getChunk(x, y)
	c.set(uint(x&(chunkSize-1)), uint(y&(chunkSize-1)), value)
}

// X and Y in 0 -> Size coordinates
func (t *Terrain) getChunk(x, y int) *chunk {
	ucx := x / chunkSize
	ucy := y / chunkSize

	// Use atomics/mutex to make sure chunk is generated
	// Basically sync.Once for each chunk but with shared mutex
	chunkPtr := (*unsafe.Pointer)(unsafe.Pointer(&t.chunks[ucx][ucy]))
	c := (*chunk)(atomic.LoadPointer(chunkPtr))

	if c == nil {
		t.mutex.Lock()
		defer t.mutex.Unlock()

		// Load again to make sure its still nil after acquiring the lock
		c = (*chunk)(atomic.LoadPointer(chunkPtr))
		if c == nil {
			// Generate chunk
			c = generateChunk(t.generator, ucx-Size/chunkSize/2, ucy-Size/chunkSize/2, nil)
			t.chunkCount++

			// Store pointer
			atomic.StorePointer(chunkPtr, unsafe.Pointer(c))
		}
	}

	return c
}

// x and y must be >= start
func (t *Terrain) start() (x, y int) {
	x = -Size / 2
	y = -Size / 2
	return
}

// x and y must be < end
func (t *Terrain) end() (x, y int) {
	x = Size / 2
	y = Size / 2
	return
}
