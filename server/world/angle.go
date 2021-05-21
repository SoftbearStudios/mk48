// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"encoding/json"
	"fmt"
	"github.com/13rac1/fastmath"
	"github.com/chewxy/math32"
)

const Pi Angle = 32768

// Angle is a 2 byte fixed-point representation of an angle.
type Angle uint16

func ToAngle(x float32) Angle {
	return Angle(x * (float32(Pi) / math32.Pi))
}

func (angle Angle) Float() float32 {
	return float32(int16(angle)) * (math32.Pi * 2 / 65536)
}

func (angle Angle) Vec2f() Vec2f {
	// ~57ns to ~30ns by using fastmath
	sin := fastmath.Sin16(uint16(angle))
	cos := fastmath.Cos16(uint16(angle))

	// Using float64 for temporary increased precision maybe isn't necessary
	return Vec2f{
		X: float32(float64(cos) * (1.0 / 32767)),
		Y: float32(float64(sin) * (1.0 / 32767)),
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
	*angle = ToAngle(f)
	return nil
}
