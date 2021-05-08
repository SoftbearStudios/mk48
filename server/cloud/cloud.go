// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package cloud

import (
	"encoding/json" // oof
	"errors"
	"mk48/server/cloud/db"
	"mk48/server/cloud/dns"
	"mk48/server/cloud/fs"
	"net"
	"sort"
	"strconv"
	"strings"
	"time"
)

const UpdatePeriod = 30 * time.Second

// A nil cloud is valid to use with any methods (acts as a no-op)
// This just means server is in offline mode
type Cloud struct {
	region     string
	serverSlot int
	ip         net.IP
	database   db.Database
	dns        dns.DNS
	fs         fs.Filesystem
}

func (cloud *Cloud) String() string {
	var builder strings.Builder
	builder.WriteByte('[')
	if cloud == nil {
		builder.WriteString("offline")
	} else {
		builder.WriteString(cloud.region)
		builder.WriteByte(' ')
		builder.WriteString(strconv.Itoa(cloud.serverSlot))
		builder.WriteByte(' ')
		builder.WriteString(cloud.ip.String())
	}
	builder.WriteByte(']')
	return builder.String()
}

// Returns nil cloud on error
func New() (*Cloud, error) {
	cloud := &Cloud{}

	userData, err := loadUserData()
	if err != nil {
		return nil, err
	}

	cloud.region = userData.Region

	cloud.ip, err = getPublicIP()
	if err != nil {
		return nil, err
	}
	session, err := getAWSSession(cloud.region)
	if err != nil {
		return nil, err
	}

	cloud.database, err = db.NewDynamoDBDatabase(session, userData.Stage)
	if err != nil {
		return nil, err
	}
	cloud.dns, err = dns.NewRoute53DNS(session, userData.Domain, userData.Route53ZoneID)
	if err != nil {
		return nil, err
	}
	cloud.fs, err = fs.NewS3Filesystem(session, userData.Stage)
	if err != nil {
		return nil, err
	}

	servers, err := cloud.database.ReadServersByRegion(cloud.region)
	if err != nil {
		return nil, err
	}

	cloud.serverSlot = -1

	// Reclaim old slot if applicable
	for _, server := range servers {
		if cloud.ip.Equal(server.IP) {
			cloud.serverSlot = server.Slot
			break
		}
	}

	// Otherwise allocate a slot
	if cloud.serverSlot == -1 {
	scan:
		for slot := 0; slot < userData.ServerSlots; slot++ {
			for _, server := range servers {
				if server.Slot == slot {
					// Slot is taken
					continue scan
				}
			}
			cloud.serverSlot = slot
			break
		}
	}

	if cloud.serverSlot == -1 {
		return nil, errors.New("no empty server slot")
	}

	err = cloud.dns.UpdateRoute(cloud.region, cloud.serverSlot, cloud.ip)
	if err != nil {
		return nil, err
	}

	err = cloud.UpdateServer(0)
	if err != nil {
		return nil, err
	}

	return cloud, nil
}

// Call at least every 30s
func (cloud *Cloud) UpdateServer(players int) error {
	if cloud == nil {
		return nil
	}
	return cloud.database.UpdateServer(db.Server{
		Region:  cloud.region,
		Slot:    cloud.serverSlot,
		IP:      cloud.ip,
		Players: players,
		TTL:     time.Now().Unix() + int64(UpdatePeriod/time.Second) + 5,
	})
}

func (cloud *Cloud) UpdateLeaderboard(playerScores map[string]int) (err error) {
	if cloud == nil {
		return nil
	}

	dbScores, err := cloud.database.ReadScores()
	if err != nil {
		return
	}

	type LeaderboardScore struct {
		Name  string `json:"name"`
		Score int    `json:"score"`
	}

	leaderboard := make(map[string][]LeaderboardScore)

	// Minimum points to affect leaderboard (to avoid inserting too many low scores)
	thresholds := make(map[string]int)

	for _, dbScore := range dbScores {
		leaderboard[dbScore.Type] = append(leaderboard[dbScore.Type], LeaderboardScore{
			Name:  dbScore.Name,
			Score: dbScore.Score,
		})
	}

	for scoreType, scores := range leaderboard {
		sort.Slice(scores, func(i, j int) bool {
			return scores[i].Score > scores[j].Score
		})

		// Leave 5 scores extra in case some expire/are moderated out
		const thresholdIndex = 15
		if len(scores) > thresholdIndex {
			thresholds[scoreType] = scores[thresholdIndex].Score
		}

		const max = 10
		if len(scores) > max {
			leaderboard[scoreType] = scores[:max]
		}
	}

	// Seconds
	now := time.Now().Unix()
	day := int64(60 * 60 * 24)
	ttlDay := now + day
	ttlWeek := now + day*7

	for name, score := range playerScores {
		if score > thresholds["single/all"] {
			err = cloud.database.UpdateScore(db.Score{
				Type:  "single/all",
				Name:  name,
				Score: score,
			})
			if err != nil {
				return
			}
		}

		if score > thresholds["single/week"] {
			err = cloud.database.UpdateScore(db.Score{
				Type:  "single/week",
				Name:  name,
				Score: score,
				TTL:   ttlWeek,
			})
			if err != nil {
				return
			}
		}

		if score > thresholds["single/day"] {
			err = cloud.database.UpdateScore(db.Score{
				Type:  "single/day",
				Name:  name,
				Score: score,
				TTL:   ttlDay,
			})
			if err != nil {
				return
			}
		}
	}

	leaderboardJSON, err := json.Marshal(leaderboard)
	if err == nil {
		_ = cloud.fs.UploadStaticFile("leaderboard.json", 10, leaderboardJSON)
	}
	return
}
