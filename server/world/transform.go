// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

type Transform struct {
	Position  Vec2f   `json:"position"`
	Velocity  float32 `json:"velocity"` // TODO omitempty crashes client
	Direction Angle   `json:"direction"`
}

// TODO: add velocity (using dot product)
func (transform Transform) Add(otherTransform Transform) Transform {
	normal := transform.Direction.Vec2f()
	transform.Position.X += otherTransform.Position.X*normal.X - otherTransform.Position.Y*normal.Y
	transform.Position.Y += otherTransform.Position.X*normal.Y + otherTransform.Position.Y*normal.X
	transform.Direction += otherTransform.Direction
	return transform
}
