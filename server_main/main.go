// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"flag"
	"fmt"
	"github.com/SoftbearStudios/mk48/server"
	"golang.org/x/net/netutil"
	"log"
	"net"
	"net/http"
	_ "net/http/pprof"
)

func main() {
	var (
		auth             string
		botMaxSpawnLevel int
		port             int
		players          int
	)

	flag.StringVar(&auth, "auth", "", "admin auth code")
	flag.IntVar(&botMaxSpawnLevel, "bot-max-spawn-level", 1, "maximum level for bots to spawn as")
	flag.IntVar(&port, "port", 8192, "http service port")
	flag.IntVar(&players, "players", 40, "minimum number of players")
	flag.Parse()

	if players < 0 {
		log.Fatal("invalid argument players: ", players)
	}

	hub := server.NewHub(players, botMaxSpawnLevel, auth)
	go hub.Run()

	if port < 0 {
		log.Println("https://mk48.io simulation started")
		// Block forever
		<-make(chan struct{})
	}

	// TODO localhost url
	log.Println("https://mk48.io server started")

	http.HandleFunc("/", hub.ServeIndex)
	http.HandleFunc("/ws", hub.ServeSocket)

	l, err := net.Listen("tcp", fmt.Sprint(":", port))

	if err != nil {
		log.Fatalf("Listen: %v", err)
	}
	defer l.Close()

	l = netutil.LimitListener(l, 256)

	log.Fatal("ListenAndServe: ", http.Serve(l, nil))
}
