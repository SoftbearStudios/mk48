package main

import (
	"bytes"
	"fmt"
	"github.com/SoftbearStudios/mk48/server"
	"github.com/SoftbearStudios/mk48/server/terrain"
	"github.com/SoftbearStudios/mk48/server/world"
	"github.com/chewxy/math32"
	"image"
	"image/color"
	"image/png"
	"os"
)

type Bot struct {
	server.ClientData
	spawned bool // only spawn once; exit program when died
}

func main() {
	hub := server.NewHub(server.HubOptions{
		Cloud:            server.Offline{},
		MinClients:       20,
		MaxBotSpawnLevel: 3,
	})

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

		img := rasterize(ship, update.Contacts, b.Hub.GetTerrain(), 1024, 128)
		var buf bytes.Buffer
		err := png.Encode(&buf, img)
		if err != nil {
			fmt.Printf("Error: %v\n", err)
			return
		}
		err = os.WriteFile("world.png", buf.Bytes(), 0755)
		if err != nil {
			fmt.Printf("Error: %v\n", err)
			return
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

// scale = meters per image dimension
// Red channel = enemy/danger
// Green channel = obstacle/land
// Blue channel = friendly/collectible
func rasterize(ship server.Contact, contacts []server.IDContact, t terrain.Terrain, scale float32, resolution int) image.Image {
	img := image.NewRGBA(image.Rect(0, 0, resolution, resolution))
	scale /= float32(resolution)

	for x := 0; x < resolution; x++ {
		for y := 0; y < resolution; y++ {
			bg := color.RGBA{A: 255}
			pos := ship.Position
			pos.X += float32(x-resolution/2) * scale
			pos.Y += float32(y-resolution/2) * scale
			if terrain.LandAtPos(t, pos) {
				bg.G = 255
			}
			img.SetRGBA(x, y, bg)
		}
	}

	for _, contact := range contacts {
		data := contact.EntityType.Data()
		normal := contact.Direction.Vec2f()
		tangent := normal.Rot90()

		var new color.RGBA
		new.A = 255

		if contact.Friendly {
			new.B = 255
		} else {
			new.R = 255 / 4
		}

		switch data.Kind {
		case world.EntityKindBoat:
			new.R *= 2
		case world.EntityKindWeapon:
			new.R *= 4
		case world.EntityKindCollectible:
			new.R = 0
			new.B = 255
		case world.EntityKindObstacle:
			new.G = 255
		}

		for l := -0.5 * data.Length; l <= 0.5*data.Length; l += scale * 0.5 {
			for w := -0.5 * data.Width; w <= 0.5*data.Width; w += scale * 0.5 {
				pos := contact.Position.Sub(ship.Position).AddScaled(normal, l).AddScaled(tangent, w)

				pos = pos.Div(scale)

				//old := rgba.RGBAAt(int(pos.X), int(pos.Y))

				img.SetRGBA(int(pos.X)+resolution/2, int(pos.Y)+resolution/2, new)
			}
		}
	}
	return img
}
