// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package db

import (
	"github.com/aws/aws-sdk-go/aws/session"
	"github.com/aws/aws-sdk-go/service/dynamodb"
	"github.com/guregu/dynamo"
)

type DynamoDBDatabase struct {
	svc          *dynamodb.DynamoDB
	db           *dynamo.DB
	scoresTable  dynamo.Table
	serversTable dynamo.Table
}

func NewDynamoDBDatabase(session *session.Session, stage string) (*DynamoDBDatabase, error) {
	ddb := &DynamoDBDatabase{svc: dynamodb.New(session)}
	ddb.db = dynamo.NewFromIface(ddb.svc)
	ddb.scoresTable = ddb.db.Table("mk48-" + stage + "-scores")
	ddb.serversTable = ddb.db.Table("mk48-" + stage + "-servers")
	return ddb, nil
}

func (ddb *DynamoDBDatabase) UpdateScore(score Score) error {
	err := ddb.scoresTable.Put(score).If("attribute_not_exists(score) OR score < ?", score.Score).Run()
	if err != nil {
		if _, ok := err.(*dynamodb.ConditionalCheckFailedException); ok {
			return nil
		}
	}
	return err
}

func (ddb *DynamoDBDatabase) ReadScores() (scores []Score, err error) {
	query := ddb.scoresTable.Scan().Iter()

	for {
		var score Score
		ok := query.Next(&score)
		if !ok {
			err = query.Err()
			return
		}
		scores = append(scores, score)
	}

	// Unreachable
	return
}

func (ddb *DynamoDBDatabase) ReadScoresByType(scoreType string) (scores []Score, err error) {
	query := ddb.scoresTable.Get("type", scoreType).Iter()

	for {
		var score Score
		ok := query.Next(&score)
		if !ok {
			err = query.Err()
			return
		}
		scores = append(scores, score)
	}

	// Unreachable
	return
}

func (ddb *DynamoDBDatabase) UpdateServer(server Server) error {
	return ddb.serversTable.Put(server).Run()
}

func (ddb *DynamoDBDatabase) ReadServers() (servers []Server, err error) {
	query := ddb.serversTable.Scan().Iter()

	for {
		var server Server
		ok := query.Next(&server)
		if !ok {
			err = query.Err()
			return
		}
		servers = append(servers, server)
	}

	// Unreachable
	return
}

func (ddb *DynamoDBDatabase) ReadServersByRegion(region string) (servers []Server, err error) {
	query := ddb.serversTable.Get("region", region).Iter()

	for {
		var server Server
		ok := query.Next(&server)
		if !ok {
			err = query.Err()
			return
		}
		servers = append(servers, server)
	}

	// Unreachable
	return
}
