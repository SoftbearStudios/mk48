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

func ClampVelocity(val, mag Velocity) Velocity {
	if val < -mag {
		return -mag
	}
	if val > mag {
		return mag
	}
	return val
}

// AddVelocityClamped adds a float to a Velocity and clamps it to mag.
func AddVelocityClamped(val Velocity, mag Velocity, amount float32) Velocity {
	// Use int64 to prevent overflow
	v := int64(val) + int64(amount*float32(MeterPerSecond))
	if v > int64(mag) {
		return mag
	}
	if v < int64(-mag) {
		return -mag
	}
	return Velocity(v)
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
