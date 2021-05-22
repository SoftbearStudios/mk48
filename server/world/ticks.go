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
	TicksMax       = Ticks(math32.MaxUint16)
)

// Ticks is a time measured in updates.
// It wraps after 65535 (109.225 minutes).
type Ticks uint16

func ToTicks(seconds float32) Ticks {
	return Ticks(seconds * float32(float64(time.Second)/float64(TickPeriod)))
}

func (ticks Ticks) Float() float32 {
	return float32(ticks) * float32(float64(TickPeriod)/float64(time.Second))
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
