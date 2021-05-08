// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package sector

import "math/bits"

// nextPowerOf2 returns the next power of 2 after or equal to n.
func nextPowerOf2(n uint16) uint16 {
	n--
	n |= n >> 1
	n |= n >> 2
	n |= n >> 4
	n |= n >> 8
	return n + 1
}

func log2(n uint16) uint8 {
	return uint8(bits.Len16(n - 1))
}
