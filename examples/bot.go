package main

import (
	"fmt"
	"github.com/SoftbearStudios/mk48/server"
	"github.com/SoftbearStudios/mk48/server/world"
	"github.com/chewxy/math32"
	"os"
)

type Bot struct {
	server.ClientData
	spawned bool // only spawn once; exit program when died
}

func main() {
	hub := server.NewHub(server.Offline{}, 0, 3, "auth")
	go hub.Run()

	bot := new(Bot)
	hub.Register(bot)

	// Block
	select {}
}

func (b *Bot) Init() {
	fmt.Println("I'm alive")
}

func (b *Bot) Close() {
	fmt.Println("I'm being closed")
	os.Exit(0)
}

func (b *Bot) Send(out server.Outbound) {
	switch update := out.(type) {
	case *server.Update:
		if update.EntityID == world.EntityIDInvalid {
			if b.spawned {
				fmt.Println("I died")
				b.Destroy()
			} else {
				fmt.Println("I'm spawning")
				b.sendToHub(server.Spawn{
					Type: world.ParseEntityType("g5"),
					Name: "BOT",
				})
				b.spawned = true
			}
			return
		}

		// Get contact of bot's own ship with linear search once.
		// Map has higher overhead so we use a slice.
		var ship server.Contact
		for i := range update.Contacts {
			if update.Contacts[i].EntityID == update.EntityID {
				ship = update.Contacts[i].Contact
				break
			}
		}

		driving := server.Manual{
			EntityID: update.EntityID,
		}

		driving.VelocityTarget = 20 * world.MeterPerSecond
		driving.DirectionTarget = world.ToAngle(math32.Pi / 2)

		fmt.Printf("I'm driving at (%.02f, %.02f) with speed %s and bearing %s\n", ship.Position.X, ship.Position.Y, ship.Velocity, ship.Direction)
		b.sendToHub(driving)
	}
}

func (b *Bot) Destroy() {
	fmt.Println("I'm being destroyed")
	b.Hub.Unregister(b)
}

func (b *Bot) Bot() bool {
	return true
}

func (b *Bot) Data() *server.ClientData {
	return &b.ClientData
}

func (b *Bot) sendToHub(inbound server.Inbound) {
	b.Hub.ReceiveSigned(server.SignedInbound{Client: b, Inbound: inbound}, false)
}
