// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package noise

func clamp(f, min, max float64) float64 {
	if f < min {
		return min
	}
	if f > max {
		return max
	}
	return f
}

func clampToByte(f float64) byte {
	if f < 0 {
		return 0
	}
	if f > 255 {
		return 255
	}
	return byte(f)
}

func max(a, b float64) float64 {
	if a > b {
		return a
	}
	return b
}
