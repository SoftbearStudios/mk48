// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package terrain

import (
	"mk48/server/world"
	"sync"
)

const (
	// Seed default seed.
	Seed = int64(1)
	// OffsetX the default x offset from the origin in world space.
	OffsetX = 256 * Scale
	// OffsetY the default y offset from the origin in world space.
	OffsetY = 256 * Scale
)

// Scale pixel width/height in meters.
// Converts from world space to terrain space.
const Scale = 25

// Source generates heightmap data.
type Source interface {
	Generate(x, y, width, height int) []byte
}

// Terrain stores the terrain of the game world.
// It calls its Source to get more heightmap data.
// All of its methods can be called concurrently (except Repair for now)
type Terrain interface {
	// At returns the heightmap data of a rect.
	At(world.AABB) *Data
	// AtPos returns the height at a point
	AtPos(world.Vec2f) byte
	// Clamp outputs the clamps the bounding box.
	Clamp(world.AABB) world.AABB
	// Collides tests if an entity collides with the Terrain.
	Collides(e *world.Entity, seconds float32) bool
	// Decodes data from this terrain into a heightmap
	Decode(*Data) ([]byte, error)
	// Modifies terrain at the position
	Sculpt(pos world.Vec2f, change float32)
	// Repairs the terrain a small amount towards its original state (prior to sculpting)
	Repair()
	// Debug prints debug info to os.StdOut.
	Debug()
}

// Data describes part of a heightmap.
type Data struct {
	world.AABB
	Data   []byte `json:"data"`   // Data is a possibly compressed terrain heightmap.
	Stride int    `json:"stride"` // Stride is width of Data.
	Length int    `json:"length"` // Length is uncompressed length of Data for faster reading.
}

var dataPool = sync.Pool{
	New: func() interface{} {
		return &Data{
			Data: make([]byte, 0, 2048),
		}
	},
}

func NewData() *Data {
	return dataPool.Get().(*Data)
}

func (data *Data) Pool() {
	*data = Data{
		Data: data.Data[:0],
	}
	dataPool.Put(data)
}
