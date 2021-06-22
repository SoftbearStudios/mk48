// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"encoding/json"
	"fmt"
	"github.com/chewxy/math32"
	"time"
)

const (
	TickPeriod     = time.Second / 10
	TicksPerSecond = Ticks(time.Second / TickPeriod)
	TicksPerDamage = 60 * TicksPerSecond // i.e. one damage takes one minute to regen
	TicksMax       = Ticks(math32.MaxUint16)
)

// Ticks is a time measured in updates.
// It wraps after 65535 (109.225 minutes or 109 torpedoes worth of damage).
type Ticks uint16

func DamageToTicks(damage float32) Ticks {
	return Ticks(damage * float32(TicksPerDamage))
}

func ToTicks(seconds float32) Ticks {
	return Ticks(seconds * float32(float64(time.Second)/float64(TickPeriod)))
}

func (ticks Ticks) Damage() float32 {
	return float32(ticks) * (1.0 / float32(TicksPerDamage))
}

func (ticks Ticks) Float() float32 {
	return float32(ticks) * float32(float64(TickPeriod)/float64(time.Second))
}

func (ticks Ticks) ClampMin(m Ticks) Ticks {
	if ticks < m {
		return m
	}
	return ticks
}

func (ticks Ticks) SaturatingSub(t Ticks) Ticks {
	if t > ticks {
		return 0
	}
	return ticks - t
}

func (ticks *Ticks) MarshalJSON() ([]byte, error) {
	return json.Marshal(ticks.Float())
}

func (ticks *Ticks) UnmarshalJSON(b []byte) error {
	var seconds float32
	if err := json.Unmarshal(b, &seconds); err != nil {
		return err
	}

	const maximum = float32(float64(math32.MaxUint16) * float64(TickPeriod) / float64(time.Second))
	if seconds > maximum || seconds < 0 {
		return fmt.Errorf("out of ticks range [0, %f): %f", maximum, seconds)
	}

	*ticks = ToTicks(seconds)
	return nil
}
