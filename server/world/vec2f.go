// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"github.com/chewxy/math32"
	"math"
)

type Vec2f struct {
	X float32 `json:"x"`
	Y float32 `json:"y"`
}

func (vec Vec2f) Mul(factor float32) Vec2f {
	vec.X *= factor
	vec.Y *= factor
	return vec
}

func (vec Vec2f) Div(divisor float32) Vec2f {
	return vec.Mul(1.0 / divisor)
}

func (vec Vec2f) AddScaled(otherVec Vec2f, factor float32) Vec2f {
	vec.X += otherVec.X * factor
	vec.Y += otherVec.Y * factor
	return vec
}

func (vec Vec2f) Add(otherVec Vec2f) Vec2f {
	vec.X += otherVec.X
	vec.Y += otherVec.Y
	return vec
}

func (vec Vec2f) Sub(otherVec Vec2f) Vec2f {
	vec.X -= otherVec.X
	vec.Y -= otherVec.Y
	return vec
}

func (vec Vec2f) Dot(otherVec Vec2f) float32 {
	return vec.X*otherVec.X + vec.Y*otherVec.Y
}

func (vec Vec2f) Angle() Angle {
	return Angle(math32.Atan2(vec.Y, vec.X))
}

// Rot90 rotates 90 degrees clockwise.
func (vec Vec2f) Rot90() Vec2f {
	return Vec2f{X: -vec.Y, Y: vec.X}
}

// Rot90 rotates 90 degrees counterclockwise.
func (vec Vec2f) RotN90() Vec2f {
	return Vec2f{X: vec.Y, Y: -vec.X}
}

// Rot180 rotates 180 degrees.
func (vec Vec2f) Rot180() Vec2f {
	return Vec2f{X: -vec.X, Y: -vec.Y}
}

func (vec Vec2f) Distance(otherVec Vec2f) float32 {
	return vec.Sub(otherVec).Length()
}

func (vec Vec2f) DistanceSquared(otherVec Vec2f) float32 {
	x := vec.X - otherVec.X
	y := vec.Y - otherVec.Y
	return x*x + y*y
}

func (vec Vec2f) Length() float32 {
	return math32.Hypot(vec.X, vec.Y)
}

func (vec Vec2f) LengthSquared() float32 {
	return vec.X*vec.X + vec.Y*vec.Y
}

func Lerp(a, b, factor float32) float32 {
	return a + (b-a)*factor
}

func (vec Vec2f) Lerp(otherVec Vec2f, factor float32) Vec2f {
	vec.X = Lerp(vec.X, otherVec.X, factor)
	vec.Y = Lerp(vec.Y, otherVec.Y, factor)
	return vec
}

func (vec Vec2f) Abs() Vec2f {
	vec.X = math32.Abs(vec.X)
	vec.Y = math32.Abs(vec.Y)
	return vec
}

func (vec Vec2f) Ceil() Vec2f {
	// Use math.Ceil instead because it uses assembly
	vec.X = float32(math.Ceil(float64(vec.X)))
	vec.Y = float32(math.Ceil(float64(vec.Y)))
	return vec
}

func (vec Vec2f) Floor() Vec2f {
	// Use math.Floor instead because it uses assembly
	vec.X = float32(math.Floor(float64(vec.X)))
	vec.Y = float32(math.Floor(float64(vec.Y)))
	return vec
}

func (vec Vec2f) Norm() Vec2f {
	return vec.Div(vec.Length())
}

func (vec Vec2f) Round() Vec2f {
	vec.X = float32(math.Round(float64(vec.X)))
	vec.Y = float32(math.Round(float64(vec.Y)))
	return vec
}
