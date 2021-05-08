// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package noise

func clampToByte(f float64) byte {
	if f < 0 {
		return 0
	}
	if f > 255 {
		return 255
	}
	return byte(f)
}
