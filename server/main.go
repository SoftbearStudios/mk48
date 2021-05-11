// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"flag"
	"fmt"
	"log"
	"net/http"
	_ "net/http/pprof"
)

func (h *Hub) serveIndex(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.Header().Set("Content-Type", "application/json")
	buf, ok := h.statusJSON.Load().([]byte)
	if ok {
		_, _ = w.Write(buf)
	}
}

func (h *Hub) serveWs(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Println("upgrade error", err)
		return
	}

	h.register <- NewSocketClient(conn)
}

func main() {
	var (
		auth    string
		port    int
		players int
	)

	flag.StringVar(&auth, "auth", "", "admin auth code")
	flag.IntVar(&port, "port", 8192, "http service port")
	flag.IntVar(&players, "players", 40, "minimum number of players")
	flag.Parse()

	if players < 0 {
		log.Fatal("invalid argument players: ", players)
	}

	hub := newHub(players, auth)
	go hub.run()

	if port < 0 {
		log.Println("https://mk48.io simulation started")
		// Block forever
		<-make(chan struct{})
	}

	// TODO localhost url
	log.Println("https://mk48.io server started")

	http.HandleFunc("/", hub.serveIndex)
	http.HandleFunc("/ws", hub.serveWs)
	log.Fatal("ListenAndServe: ", http.ListenAndServe(fmt.Sprint(":", port), nil))
}
