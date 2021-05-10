// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"math/rand"
	"mk48/server/world"
	"sort"
	"strconv"
	"testing"
)

func createPlayerSet(n int) world.PlayerSet {
	random := rand.New(rand.NewSource(0))

	set := make(world.PlayerSet, n)
	for i := range set {
		score := int(rand.NormFloat64()*30 + 10)
		if score < 0 {
			score = 0
		}

		set[i] = &world.Player{
			PlayerData: world.PlayerData{
				Name:   randomBotName(random),
				Score:  score,
				TeamID: 0,
			},
		}
	}
	return set
}

func benchLeaderboardFunc(b *testing.B, f func(world.PlayerSet, int) []world.PlayerData, n, count int) {
	set := createPlayerSet(n)

	b.Run(strconv.Itoa(n), func(b *testing.B) {
		b.StopTimer()
		b.ReportAllocs()

		s := make(world.PlayerSet, len(set))

		for i := 0; i < b.N; i++ {
			copy(s, set)
			b.StartTimer()

			top := f(s, count)

			b.StopTimer()
			sorted := sort.SliceIsSorted(top, func(i, j int) bool {
				return top[i].ScoreLess(&top[j])
			})
			if !sorted {
				b.Errorf("not sorted: %v", top)
			}
		}

		b.StartTimer()
	})
}

func BenchmarkTop10PlayersHeap(b *testing.B) {
	for i := 64; i <= 4096; i *= 2 {
		benchLeaderboardFunc(b, topPlayersHeap, i, 10)
	}
}

func BenchmarkTop10PlayersInsert(b *testing.B) {
	for i := 64; i <= 4096; i *= 2 {
		benchLeaderboardFunc(b, topPlayersInsert, i, 10)
	}
}
