// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"encoding/json"
	"fmt"
	"github.com/chewxy/math32"
	"math"
)

const (
	// MeterPerSecond is 1 m/s
	MeterPerSecond Velocity = 1 << 5
	VelocityMax             = math32.MaxInt16 / float32(MeterPerSecond)
	VelocityMin             = math32.MinInt16 / float32(MeterPerSecond)
)

// Velocity is a 11_5 fixed point representing any valid velocity.
type Velocity int16

// ToVelocity converts a float in m/s to a Velocity.
func ToVelocity(x float32) Velocity {
	// math.Floor is much faster than math32.Floor.
	return Velocity(math.Floor(float64(x * float32(MeterPerSecond))))
}

// Float returns the Velocity as a float in m/s.
func (vel Velocity) Float() float32 {
	return float32(vel) * (1.0 / (float32(MeterPerSecond)))
}

func (vel Velocity) ClampMagnitude(mag Velocity) Velocity {
	if vel < -mag {
		return -mag
	}
	if vel > mag {
		return mag
	}
	return vel
}

func (vel Velocity) ClampMin(min Velocity) Velocity {
	if vel < 0 {
		if vel > -min {
			return -min
		}
	} else if vel < min {
		return min
	}
	return vel
}

// AddClamped adds a float to a Velocity and clamps it to mag.
func (vel Velocity) AddClamped(amount float32, mag Velocity) Velocity {
	// Use int64 to prevent overflow
	v := int64(vel) + int64(amount*float32(MeterPerSecond))
	if v > int64(mag) {
		return mag
	}
	if v < int64(-mag) {
		return -mag
	}
	return Velocity(v)
}

func (vel Velocity) String() string {
	return fmt.Sprintf("%.01f m/s", vel.Float())
}

func (vel Velocity) MarshalJSON() ([]byte, error) {
	return json.Marshal(vel.Float())
}

func (vel *Velocity) UnmarshalJSON(b []byte) error {
	var f float32
	if err := json.Unmarshal(b, &f); err != nil {
		return err
	}
	if f < VelocityMin || f > VelocityMax {
		return fmt.Errorf("velocity out of range [%f, %f]: %f", VelocityMin, VelocityMax, f)
	}
	*vel = ToVelocity(f)
	return nil
}
