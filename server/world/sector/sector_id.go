// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package sector

import (
	"github.com/chewxy/math32"
	"github.com/SoftbearStudios/mk48/server/world"
)

type (
	// sectorID is a unique identifier of a sector by is position in the World
	sectorID struct {
		x, y int16
	}

	// sectorIndex is a pointer to where a sectorEntity is stored
	sectorIndex struct {
		sectorID
		index int32 // use int32 so sectorIndex can be 8 bytes instead of 16
	}
)

func (id sectorID) add(otherSectorID sectorID) sectorID {
	id.x += otherSectorID.x
	id.y += otherSectorID.y
	return id
}

// min sets the sectorID's min value
func (id sectorID) min(m int16) sectorID {
	if id.x < m {
		id.x = m
	}
	if id.y < m {
		id.y = m
	}
	return id
}

// max sets the sectorID's max value
func (id sectorID) max(m int16) sectorID {
	if id.x > m {
		id.x = m
	}
	if id.y > m {
		id.y = m
	}
	return id
}

func (id sectorID) inRadius(position world.Vec2f, radius float32) bool {
	distance := world.Vec2f{
		X: math32.Abs(float32(id.x)*size + size/2 - position.X),
		Y: math32.Abs(float32(id.y)*size + size/2 - position.Y),
	}

	if distance.X > size/2+radius || distance.Y > size/2+radius {
		return false
	}

	if distance.X <= size/2 || distance.Y <= size/2 {
		return true
	}

	cornerDistance := world.Vec2f{X: distance.X - size/2, Y: distance.Y - size/2}.LengthSquared()
	return cornerDistance < radius*radius
}

func (id sectorID) sliceIndex(width uint16) int {
	min := -int16(width / 2)
	max := int16(width / 2)

	if id.x < min || id.x >= max || id.y < min || id.y >= max {
		return -1
	}

	x := int(id.x - min)
	y := int(id.y - min)

	return x + y*int(width)
}

// sliceIndexSectorID returns a sectorID from a slice index.
// width must be a power of 2.
func sliceIndexSectorID(index int, width uint16, logWidth uint8) sectorID {
	return sectorID{x: int16(int(uint16(index)&(width-1)) - int(width/2)), y: int16((index >> logWidth) - int(width/2))}
}

func vec2fSectorID(vec world.Vec2f) sectorID {
	s := vec.Mul(1.0 / size).Floor()
	return sectorID{x: int16(s.X), y: int16(s.Y)}
}
