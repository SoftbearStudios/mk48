// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"math/rand"
	"time"
)

func min(a, b float32) float32 {
	if a < b {
		return a
	}
	return b
}

func max(a, b float32) float32 {
	if a > b {
		return a
	}
	return b
}

func clamp(val, minimum, maximum float32) float32 {
	return min(max(val, minimum), maximum)
}

func clampMagnitude(val, mag float32) float32 {
	return clamp(val, -mag, mag)
}

func minInt(a, b int) int {
	if a < b {
		return a
	}
	return b
}

func maxInt(a, b int) int {
	if a > b {
		return a
	}
	return b
}

func square(a float32) float32 {
	return a * a
}

func mapRanges(number, oldMin, oldMax, newMin, newMax float32, clampToRange bool) float32 {
	oldRange := oldMax - oldMin
	newRange := newMax - newMin
	numberNormalized := (number - oldMin) / oldRange
	mapped := newMin + numberNormalized*newRange
	if clampToRange {
		mapped = clamp(mapped, newMin, newMax)
	}
	return mapped
}

func unixMillis() int64 {
	return time.Now().UnixNano() / (time.Millisecond.Nanoseconds() / time.Nanosecond.Nanoseconds())
}

func copyFloats(a []float32) []float32 {
	b := make([]float32, len(a))
	copy(b, a)
	return b
}

func copyAngles(a []Angle) []Angle {
	b := make([]Angle, len(a))
	copy(b, a)
	return b
}

// Returns random alphanumeric string of length n
func RandString(n int) string {
	const letterBytes = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"

	b := make([]byte, n)
	for i := range b {
		b[i] = letterBytes[rand.Intn(len(letterBytes))]
	}
	return string(b)
}
