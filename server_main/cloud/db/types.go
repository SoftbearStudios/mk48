// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package db

import (
	"net"
)

type Score struct {
	Type  string `dynamo:"type"`
	Name  string `dynamo:"name"`
	Score int    `dynamo:"score"`
	TTL   int64  `dynamo:"ttl,omitempty"`
}

type Server struct {
	Region  string `dynamo:"region"`
	Slot    int    `dynamo:"slot"`
	IP      net.IP `dynamo:"ip"`
	Players int    `dynamo:"players"`
	TTL     int64  `dynamo:"ttl,omitempty"`
}

type Statistic struct {
	Region string `dynamo:"region"`

	// Unix millis; should be aligned to the hour
	Timestamp int64 `dynamo:"timestamp"`

	// Atomic counters
	Plays      int `dynamo:"plays"`      // i.e. each spawn
	Players    int `dynamo:"players"`    // i.e. each connection
	NewPlayers int `dynamo:"newPlayers"` // i.e. each "new" spawn
}
