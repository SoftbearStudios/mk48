// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package compressed

import (
	"mk48/server/terrain"
	"mk48/server/world"
)

func clampToGrassByte(f float32) byte {
	if f < 0 {
		return 0
	}
	if f > terrain.GrassLevel {
		return terrain.GrassLevel
	}
	return byte(f)
}

// Rounds a byte to 4 bits of precision
func roundByte(b byte) byte {
	return b & 0b11110000
}

// blerp does bi-linear interpolation on 4 bytes given the tx and ty offsets.
func blerp(c00, c10, c01, c11 byte, tx, ty float32) byte {
	return byte(world.Lerp(
		world.Lerp(float32(c00), float32(c10), tx),
		world.Lerp(float32(c01), float32(c11), tx),
		ty,
	))
}

func minInt(x, y int) int {
	if x < y {
		return x
	}
	return y
}

func maxInt(x, y int) int {
	if x > y {
		return x
	}
	return y
}

func min(a, b float32) float32 {
	if a < b {
		return a
	}
	return b
}
