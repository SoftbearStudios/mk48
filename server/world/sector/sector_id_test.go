// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package sector

import (
	"math/rand"
	"testing"
)

func TestSectorID_sliceIndex(t *testing.T) {
	const width = 1 << 8

	errors := 0

	for i := 0; i < 10000; i++ {
		x := int16(rand.Intn(width) - width/2)
		y := int16(rand.Intn(width) - width/2)
		id := sectorID{x: x, y: y}

		index := id.sliceIndex(width)
		newID := sliceIndexSectorID(index, width, log2(width))

		if id != newID {
			t.Errorf("sliceIndexSectorID(%#v.sliceIndex(width), width) != %#v", id, newID)
			errors++
			if errors > 10 {
				t.FailNow()
			}
		}
	}
}
