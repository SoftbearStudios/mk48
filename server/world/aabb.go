// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

type AABB struct {
	Vec2f
	Width  float32 `json:"width"`
	Height float32 `json:"height"`
}

func AABBFrom(x, y, width, height float32) AABB {
	return AABB{
		Vec2f:  Vec2f{X: x, Y: y},
		Width:  width,
		Height: height,
	}
}

// Intersects a and b are intersecting
func (a AABB) Intersects(b AABB) bool {
	return a.X+a.Width >= b.X && a.X <= b.X+b.Width && a.Y+a.Height >= b.Y && a.Y <= b.Height+b.Y
}

// Contains a fully contains b
func (a AABB) Contains(b AABB) bool {
	return a.X <= b.X && a.Y <= b.Y && a.X+a.Width >= b.X+b.Width && a.Y+a.Height >= b.Y+b.Height
}

// CornerCoordinates Center coords to corner coords
func (a AABB) CornerCoordinates() AABB {
	a.Vec2f = Vec2f{X: a.X - a.Width*0.5, Y: a.Y - a.Height*0.5}
	return a
}

// Quadrants All quadrants of a
func (a AABB) Quadrants() [4]AABB {
	var quadrants [4]AABB
	for i := range quadrants {
		quadrants[i] = a.Quadrant(i)
	}
	return quadrants
}

// Quadrant of a by index
func (a AABB) Quadrant(quadrant int) AABB {
	pos := a.Vec2f
	width := a.Width * 0.5
	height := a.Height * 0.5
	switch quadrant {
	case 1:
		pos.X += width
	case 2:
		pos.X += width
		pos.Y += height
	case 3:
		pos.Y += height
	}
	return AABB{Vec2f: pos, Width: width, Height: height}
}
