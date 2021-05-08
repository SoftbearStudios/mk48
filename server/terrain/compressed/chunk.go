// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package compressed

import (
	"mk48/server/terrain"
	"sync/atomic"
)

// chunkSize is the width and height of a chunk.
// It must be a power of 2.
const chunkSize = 64

const regenMillis = 30 * 60 * 1000

// chunk stores a region of heightmap data as nibbles.
type chunk struct {
	data  [chunkSize][chunkSize / (2 * 8)]uint64
	regen int64 // timestamp of next regen (managed by compressed.Repair)
}

// If c passed in, it is partially regenerated (atomically)
func generateChunk(generator terrain.Source, cx, cy int, c *chunk) *chunk {
	heightmap := generator.Generate(cx*chunkSize, cy*chunkSize, chunkSize, chunkSize)

	// Early bounds check
	_ = heightmap[chunkSize*chunkSize-1]

	if c == nil {
		c = new(chunk)

		for i := 0; i < chunkSize; i++ {
			for j := 0; j < chunkSize; j++ {
				height := heightmap[i*chunkSize+j]
				c.data[i][j/16] |= uint64(height>>4) << ((j % 16) * 4)
			}
		}
	} else {
		for i := uint(0); i < chunkSize; i++ {
			for j := uint(0); j < chunkSize; j++ {
				// TODO: Atomically set one full integer at a time, not 16 nibbles
				height := heightmap[i*chunkSize+j] & 0b11110000
				oldHeight := c.at(j, i)
				if height > oldHeight {
					c.set(j, i, oldHeight+0b10000)
				} else if height < oldHeight {
					c.set(j, i, oldHeight-0b10000)
				}
			}
		}
	}

	return c
}

// at gets a relative position in the chunk.
func (c *chunk) at(x, y uint) byte {
	dat := atomic.LoadUint64(&c.data[y][x/16])
	return (byte(dat>>((x%16)*4)) & 0b1111) << 4
}

// set sets a relative position's value.
func (c *chunk) set(x, y uint, value byte) {
	shift := (x % 16) * 4
	addr := &c.data[y][x/16]

	for {
		oldVal := atomic.LoadUint64(addr)
		newVal := (oldVal & ^(0b1111 << shift)) | uint64(value>>4)<<shift
		if atomic.CompareAndSwapUint64(addr, oldVal, newVal) {
			break
		}
	}
}
