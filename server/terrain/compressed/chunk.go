// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package compressed

import (
	"github.com/SoftbearStudios/mk48/server/terrain"
)

const (
	// chunkSizeBits is the chunk size in bits.
	chunkSizeBits = 6
	// chunkSize is the width and height of a chunk.
	// It must be a power of 2.
	chunkSize = 1 << chunkSizeBits
)

const regenMillis = 30 * 60 * 1000

// chunk stores a region of heightmap data as nibbles.
type chunk struct {
	data  [chunkSize][chunkSize / 2]byte
	regen int64 // timestamp of next regen (managed by compressed.Repair)
}

// If c passed in, it is partially regenerated (atomically)
func generateChunk(generator terrain.Source, cx, cy uint, c *chunk) *chunk {
	heightmap := generator.Generate(int(cx*chunkSize), int(cy*chunkSize), chunkSize, chunkSize)

	// Early bounds check
	_ = heightmap[chunkSize*chunkSize-1]

	if c == nil {
		c = new(chunk)

		for i := uint(0); i < chunkSize; i++ {
			for j := uint(0); j < chunkSize; j++ {
				c.set(j, i, heightmap[i*chunkSize+j])
			}
		}
	} else {
		for i := uint(0); i < chunkSize; i++ {
			for j := uint(0); j < chunkSize; j++ {
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

// at gets a global position in the chunk.
// It assumes c is the correct chunk.
func (c *chunk) at(x, y uint) byte {
	// Convert to relative coords.
	sx := (x / 2) & (chunkSize/2 - 1)
	y &= chunkSize - 1

	return (c.data[y][sx] << ((x & 1) * 4)) & 0b11110000
}

// set sets a global position's value.
// It assumes c is the correct chunk.
func (c *chunk) set(x, y uint, value byte) {
	// Convert to relative coords.
	sx := (x / 2) & (chunkSize/2 - 1)
	y &= chunkSize - 1

	shift := (x & 1) * 4
	c.data[y][sx] = (c.data[y][sx] & (0b1111 << shift)) | ((value & 0b11110000) >> shift)
}
