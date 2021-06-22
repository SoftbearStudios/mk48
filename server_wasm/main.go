// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"github.com/SoftbearStudios/mk48/server"
	"log"
)

func main() {
	hub := server.NewHub(server.HubOptions{
		Cloud:            server.Offline{},
		MinClients:       20,
		MaxBotSpawnLevel: 3,
	})

	log.Println("https://mk48.io WASM server started")

	hub.Register(&localClient)

	hub.Run()
}
