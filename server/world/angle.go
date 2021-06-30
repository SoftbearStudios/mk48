// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"encoding/json"
	"fmt"
	"github.com/chewxy/math32"
	"math/rand"
)

const Pi = 32768

// Angle is a 2 byte fixed-point representation of an angle.
// To convert a float in radians to an Angle use ToAngle.
type Angle uint16

// ToAngle converts a float in the range of an [MinInt32, MaxInt32] to an angle.
func ToAngle(x float32) Angle {
	x *= Pi / math32.Pi
	return Angle(int16(int32(x)))
}

// Returns a random angle (all possible angles are equally likely)
func RandomAngle() Angle {
	return Angle(rand.Intn(2 * Pi))
}

// Float returns angle in range [-π, π)
func (angle Angle) Float() float32 {
	return float32(int16(angle)) * (math32.Pi / Pi)
}

func (angle Angle) Vec2f() Vec2f {
	// Converting to range [0, 2π) instead of [-π, π) increases speed by 10%.
	sin, cos := math32.Sincos(float32(angle) * (math32.Pi / Pi))
	return Vec2f{
		X: cos,
		Y: sin,
	}
}

func (angle Angle) ClampMagnitude(m Angle) Angle {
	if int16(angle) < -int16(m) {
		return -m
	}
	if int16(angle) > int16(m) {
		return m
	}
	return angle
}

func (angle Angle) Diff(otherAngle Angle) (difference Angle) {
	return angle - otherAngle
}

func (angle Angle) Lerp(otherAngle Angle, factor float32) Angle {
	return angle + ToAngle(otherAngle.Diff(angle).Float()*factor)
}

func (angle Angle) Abs() float32 {
	return math32.Abs(angle.Float())
}

func (angle Angle) Inv() Angle {
	return angle + Pi
}

func (angle Angle) String() string {
	return fmt.Sprintf("%.01f degrees", angle.Float()*(180/math32.Pi))
}

func (angle Angle) MarshalJSON() ([]byte, error) {
	return json.Marshal(angle.Float())
}

func (angle *Angle) UnmarshalJSON(b []byte) error {
	var f float32
	if err := json.Unmarshal(b, &f); err != nil {
		return err
	}
	// Use nextafter to always round down (important as 3.141592653589793 must
	// not round to 3.1415927, which exceeds PI, and wraps to a -PI angle)
	*angle = ToAngle(math32.Nextafter(f, 0))
	return nil
}
