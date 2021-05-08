// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"fmt"
	"github.com/chewxy/math32"
)

type Angle float32

func (angle Angle) Vec2f() Vec2f {
	sin, cos := math32.Sincos(float32(angle))
	return Vec2f{
		X: cos,
		Y: sin,
	}
}

func (angle Angle) ClampMagnitude(max Angle) Angle {
	if angle < -max {
		return -max
	}
	if angle > max {
		return max
	}
	return angle
}

func (angle Angle) Diff(otherAngle Angle) (difference Angle) {
	difference = angle - otherAngle
	const mod = Angle(math32.Pi * 2)

	// Early check speeds it up from 25ns to 8ns
	if difference >= mod || difference < -mod {
		difference = Angle(math32.Mod(float32(difference), float32(mod)))
	}

	if difference < Angle(-math32.Pi) {
		difference += Angle(math32.Pi * 2)
	} else if difference >= Angle(math32.Pi) {
		difference -= Angle(math32.Pi * 2)
	}
	return
}

func (angle Angle) Lerp(otherAngle Angle, factor float32) Angle {
	delta := otherAngle.Diff(angle)
	return angle + delta*Angle(factor)
}

func (angle Angle) Abs() Angle {
	return Angle(math32.Abs(float32(angle)))
}

func (angle Angle) Inv() Angle {
	return angle + Angle(math32.Pi)
}

func (angle Angle) String() string {
	return fmt.Sprintf("%.01f degrees", float32(angle)*180/math32.Pi)
}
