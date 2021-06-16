// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package terrain

import (
	"github.com/SoftbearStudios/mk48/server/world"
	"sync"
)

/*
	List of curated seeds/offsets:
		1, 256, 256
		46, 0, 128
		48, 0, 64
		56, -128, -128
*/

const (
	// Seed default seed.
	Seed = int64(56)
	// OffsetX the default x offset from the origin in world space.
	OffsetX = -128 * Scale
	// OffsetY the default y offset from the origin in world space.
	OffsetY = -128 * Scale
)

// Scale pixel width/height in meters.
// Converts from world space to terrain space.
const Scale = 25

// Source generates heightmap data.
type Source interface {
	Generate(x, y, width, height int) []byte
}

// Data describes part of a heightmap.
// It may be in a compressed format.
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
