// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

import (
	"container/heap"
	"github.com/SoftbearStudios/mk48/server/world"
	"sort"
	"time"
)

// Leaderboard sends Leaderboard message to each Client.
// Its run in parallel because it doesn't write to World
func (h *Hub) Leaderboard() {
	defer h.timeFunction("leaderboard", time.Now())

	playerSet := make(world.PlayerSet, 0, h.clients.Len)
	for client := h.clients.First; client != nil; client = client.Data().Next {
		player := &client.Data().Player
		if player.EntityID == world.EntityIDInvalid {
			continue
		}
		playerSet = append(playerSet, &player.Player)
	}

	top := TopPlayers(playerSet, 10)
	leaderboard := outbound(Leaderboard{Leaderboard: top})

	for client := h.clients.First; client != nil; client = client.Data().Next {
		client.Send(leaderboard)
	}
}

// TopPlayers Top count players with highest score of a world.PlayerSet.
func TopPlayers(players world.PlayerSet, count int) []world.PlayerData {
	if count <= 20 {
		return topPlayersInsert(players, count)
	} else {
		return topPlayersHeap(players, count)
	}
}

// topPlayersHeap Uses heap to get top count players.
// It has a time complexity of O(n + m * log(n)).
func topPlayersHeap(players world.PlayerSet, count int) []world.PlayerData {
	heap.Init(&players)

	top := make([]world.PlayerData, 0, count)
	for players.Len() > 0 && len(top) < cap(top) {
		player := heap.Pop(&players).(*world.Player)
		top = append(top, player.PlayerData)
	}

	return top
}

// topPlayersInsert Uses insertion to get top count players.
// It has a time complexity of O(n * m).
func topPlayersInsert(players world.PlayerSet, count int) []world.PlayerData {
	n := len(players)
	if count > n {
		count = n
	}

	// Insert into subset
	subset := players[:count]
	sort.Sort(&subset)

	if count < n {
		set := players[count:]
		end := len(subset) - 1

		for _, p := range set {
			j := end
			if !p.ScoreLess(&subset[j].PlayerData) {
				continue
			}
			subset[j] = p

			for ; j > 0 && subset[j].ScoreLess(&subset[j-1].PlayerData); j-- {
				subset.Swap(j, j-1)
			}
		}
	}

	top := make([]world.PlayerData, len(subset))
	for i, p := range subset {
		top[i] = p.PlayerData
	}

	return top
}
