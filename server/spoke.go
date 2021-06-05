package server

import (
	"fmt"
	"github.com/SoftbearStudios/mk48/server/terrain"
	"github.com/gorilla/websocket"
	"log"
	"net/url"
	"time"
)

type (
	SpokeOptions struct {
		URL url.URL
	}

	spokeConnection struct {
		conn *websocket.Conn
		send chan Inbound
	}

	Spoke struct {
		options     SpokeOptions
		connections map[Client]spokeConnection
		terrain     terrain.Terrain
	}
)

func NewSpoke(options SpokeOptions) *Spoke {
	return &Spoke{
		options: options,
	}
}

func (s *Spoke) Register(client Client) (err error) {
	conn, _, err := websocket.DefaultDialer.Dial(s.options.URL.String(), nil)
	if err == nil {
		s.connections[client] = spokeConnection{
			conn: conn,
			send: make(chan Inbound),
		}
	}
	return
}

func (s *Spoke) ReceiveSigned(in SignedInbound) {
	sc := s.connections[in.Client]
	if sc.conn == nil {
		panic("Cannot receive from client that was never registered")
	}

	w, err := sc.conn.NextWriter(websocket.TextMessage)
	if err != nil {
		// TODO
	}

	// Wrap with Message to marshal type
	if err = json.NewEncoder(w).Encode(Message{Data: in}); err != nil {
		// TODO
	}

	go s.readPump(in.Client)
	go s.writePump(in.Client)
}

func (s *Spoke) GetTerrain() terrain.Terrain {
	return s.terrain
}

func (s *Spoke) Unregister(client Client) {
	sc, ok := s.connections[client]
	if !ok {
		panic("Cannot unregister client that was never registered")
	}
	err := sc.conn.WriteMessage(websocket.CloseMessage, websocket.FormatCloseMessage(websocket.CloseNormalClosure, ""))
	if err == nil {
		time.Sleep(time.Second / 4)
	}
	sc.conn.Close()
	delete(s.connections, client)
}

func (s *Spoke) readPump(client Client) {
	sc := s.connections[client]

	defer client.Destroy()
	sc.conn.SetReadLimit(maxMessageSize)
	_ = sc.conn.SetReadDeadline(time.Now().Add(pongWait))
	sc.conn.SetPongHandler(func(string) error {
		_ = sc.conn.SetReadDeadline(time.Now().Add(pongWait))
		return nil
	})

	for {
		_, r, err := sc.conn.NextReader()
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				log.Println("close error:", err)
			}
			break
		}

		var message Message
		err = json.NewDecoder(r).Decode(&message)
		if err != nil {
			log.Println("unmarshal error:", err.Error())
			break
		}

		if invalidMessage, ok := message.Data.(InvalidInbound); ok {
			log.Println("invalid message type received:", invalidMessage.messageType)
		} else {
			client.Send(message.Data.(Outbound))
		}
	}
}

func (s *Spoke) writePump(client Client) {
	sc := s.connections[client]
	pingTicker := time.NewTicker(pingPeriod)

	defer func() {
		if err := recover(); err != nil {
			if debugSocket {
				fmt.Println("send error:", err)
			}
		}
		pingTicker.Stop()
		client.Destroy()
	}()

	for {
		select {
		case out, ok := <-sc.send:
			_ = sc.conn.SetWriteDeadline(time.Now().Add(writeWait))
			if !ok {
				// The hub closed the channel.
				_ = sc.conn.WriteMessage(websocket.CloseMessage, nil)
				panic("hub closed channel")
			}

			w, err := sc.conn.NextWriter(websocket.TextMessage)
			if err != nil {
				panic(err)
			}

			// Wrap with Message to marshal type
			if err = json.NewEncoder(w).Encode(Message{Data: out}); err != nil {
				panic(err)
			}

			if err = w.Close(); err != nil {
				panic(err)
			}
		case <-pingTicker.C:
			_ = sc.conn.SetWriteDeadline(time.Now().Add(writeWait))
			if err := sc.conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}
		}
	}
}
