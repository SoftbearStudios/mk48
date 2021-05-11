// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"math/rand"
	"sync"
	"time"
)

func init() {
	rand.Seed(time.Now().UnixNano())
}

var randPool = sync.Pool{
	New: func() interface{} {
		return rand.New(rand.NewSource(rand.Int63()))
	},
}

func getRand() *rand.Rand {
	return randPool.Get().(*rand.Rand)
}

func poolRand(r *rand.Rand) {
	randPool.Put(r)
}

// prob has a p probability of returning true.
// Uses float64 for small probabilities.
func prob(r *rand.Rand, p float64) bool {
	return r.Float64() < p
}

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

func clampMagnitude(val, maximum float32) float32 {
	return min(max(val, -maximum), maximum)
}

func square(a float32) float32 {
	return a * a
}

func invSquare(a float32) float32 {
	square := a * a
	if square == 0 {
		return 0
	}
	return 1.0 / square
}

func unixMillis() int64 {
	return time.Now().UnixNano() / int64(time.Millisecond/time.Nanosecond)
}
