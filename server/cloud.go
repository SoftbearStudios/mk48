// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

import (
	"fmt"
	"time"
)

// A nil cloud is valid to use with any methods (acts as a no-op)
// This just means server is in offline mode
type Cloud interface {
	fmt.Stringer
	UpdateServer(players int) error
	IncrementPlayerStatistic()
	IncrementNewPlayerStatistic()
	IncrementPlaysStatistic()
	FlushStatistics() error
	UpdateLeaderboard(playerScores map[string]int) error
	UploadTerrainSnapshot(data []byte) error // takes an encoded PNG
	UpdatePeriod() time.Duration
}

type Offline struct{}

func (offline Offline) String() string {
	return "offline"
}

func (offline Offline) UpdateServer(players int) error {
	return nil
}

func (offline Offline) IncrementPlayerStatistic()    {}
func (offline Offline) IncrementNewPlayerStatistic() {}
func (offline Offline) IncrementPlaysStatistic()     {}

func (offline Offline) FlushStatistics() error {
	return nil
}

func (offline Offline) UpdateLeaderboard(playerScores map[string]int) error {
	return nil
}

func (offline Offline) UploadTerrainSnapshot(data []byte) error {
	return nil
}

func (offline Offline) UpdatePeriod() time.Duration {
	return time.Hour
}

func (h *Hub) Cloud() {
	fmt.Println("Updating cloud")

	err := h.cloud.FlushStatistics()
	if err != nil {
		fmt.Println("Error flushing statistics:", err)
	}

	playerCount := 0

	// Note: Cannot use to determine number of players, as long as there
	// are duplicate names
	playerScores := make(map[string]int)

	for client := h.clients.First; client != nil; client = client.Data().Next {
		if !client.Bot() {
			playerCount++
			player := &client.Data().Player
			if player.Score > playerScores[player.Name] {
				playerScores[player.Name] = player.Score
			}
		}
	}

	go func() {
		err := h.cloud.UpdateLeaderboard(playerScores)
		if err != nil {
			fmt.Println("Error updating leaderboard:", err)
		}
	}()

	statusJSON, err := json.Marshal(struct {
		Players int `json:"players"`
	}{
		Players: playerCount,
	})

	if err == nil {
		h.statusJSON.Store(statusJSON)
	} else {
		fmt.Println("error marshaling status:", err)
	}

	err = h.cloud.UpdateServer(playerCount)
	if err != nil {
		fmt.Println("Error updating server:", err)
	}
}
