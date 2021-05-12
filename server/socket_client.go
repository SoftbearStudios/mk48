// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"fmt"
	"log"
	"mk48/server/terrain"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

const (
	// Time allowed to write a message to the peer.
	writeWait = 5 * time.Second

	// Time allowed to read the next pong message from the peer.
	pongWait = 60 * time.Second

	// Send pings to peer with this period. Must be less than pongWait.
	pingPeriod = (pongWait * 9) / 10

	// Maximum message size allowed from peer.
	maxMessageSize = 4096

	debugSocket = false
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true // TODO: Read domain env var and actually enforce similarity
	},
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
}

// SocketClient is a middleman between the websocket connection and the hub.
type SocketClient struct {
	ClientData
	conn *websocket.Conn
	send chan outbound
	once sync.Once
}

// Create a SocketClient from a connection
func NewSocketClient(conn *websocket.Conn) *SocketClient {
	return &SocketClient{
		conn: conn,
		send: make(chan outbound, 16), // Allows ~1.5 seconds of messages to backup before close
	}
}

func (client *SocketClient) Close() {
	close(client.send)
}

func (client *SocketClient) Data() *ClientData {
	return &client.ClientData
}

func (client *SocketClient) Destroy() {
	client.once.Do(func() {
		hub := client.Hub

		// Needs to go through when called on hub goroutine.
		select {
		case hub.unregister <- client:
		default:
			go func() {
				hub.unregister <- client
			}()
		}

		_ = client.conn.Close()
	})
}

func (client *SocketClient) Init(_ terrain.Terrain) {
	go client.writePump()
	go client.readPump()
}

func (client *SocketClient) Send(message outbound) {
	select {
	case client.send <- message:
	default:
		// Not responsive
		client.Destroy()
	}
}

func (client *SocketClient) readPump() {
	defer client.Destroy()
	client.conn.SetReadLimit(maxMessageSize)
	_ = client.conn.SetReadDeadline(time.Now().Add(pongWait))
	client.conn.SetPongHandler(func(string) error {
		_ = client.conn.SetReadDeadline(time.Now().Add(pongWait))
		return nil
	})

	for {
		_, r, err := client.conn.NextReader()
		if err != nil {
			if debugSocket {
				fmt.Println(err)
			}
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
			client.Hub.inbound <- SignedInbound{Client: client, inbound: message.Data.(inbound)}
		}
	}
}

func (client *SocketClient) writePump() {
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
		case out, ok := <-client.send:
			_ = client.conn.SetWriteDeadline(time.Now().Add(writeWait))
			if !ok {
				// The hub closed the channel.
				_ = client.conn.WriteMessage(websocket.CloseMessage, nil)
				return
			}

			w, err := client.conn.NextWriter(websocket.TextMessage)
			if err != nil {
				panic(err)
			}

			// Wrap with Message to marshal type
			if err = json.NewEncoder(w).Encode(Message{Data: out}); err != nil {
				log.Println("encoding error:", err)
				panic(err)
			}

			out.Pool()

			if err = w.Close(); err != nil {
				panic(err)
			}
		case <-pingTicker.C:
			_ = client.conn.SetWriteDeadline(time.Now().Add(writeWait))
			if err := client.conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}
		}
	}
}
