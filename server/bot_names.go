// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	_ "embed"
	"math/rand"
	"strings"
)

//go:embed names.txt
var botNamesRaw string

//go:embed team-names.txt
var botTeamNamesRaw string

var (
	botNames     = strings.Split(strings.ToLower(botNamesRaw), "\n")
	botTeamNames = strings.Split(strings.ToLower(botTeamNamesRaw), "\n")
)

func randomBotName(r *rand.Rand) (name string) {
	for name == "" {
		name = botNames[r.Intn(len(botNames))]
	}

	if prob(r, 0.1) {
		name = strings.ToUpper(name)
	}
	return
}

func randomTeamName(r *rand.Rand) (name string) {
	for name == "" {
		name = botTeamNames[r.Intn(len(botTeamNames))]
	}
	return name
}
