// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package db

type Database interface {
	UpdateScore(score Score) error
	ReadScores() (scores []Score, err error)
	ReadScoresByType(scoreType string) (scores []Score, err error)
	UpdateServer(server Server) error
	ReadServers() (servers []Server, err error)
	ReadServersByRegion(region string) (servers []Server, err error)
}
