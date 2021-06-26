// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

import (
	"fmt"
	"log"
	"net"
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
	pingPeriod = (pongWait * 8) / 10

	// If more than this many messages are queued for sending, the
	// socket is congested and messages may be dropped
	socketCongestionThreshold = 5

	// Allows ~1 second of messages to backup before close
	// (although the sending may be throttled to slow down
	// hitting this limit)
	socketBufferSize = 16

	// Maximum message size allowed from peer.
	maxMessageSize = 512

	debugSocket = true
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true // TODO: Read domain env var and actually enforce similarity
	},
	HandshakeTimeout: time.Second,
	ReadBufferSize:   maxMessageSize,
	WriteBufferSize:  2048,
}

// SocketClient is a middleman between the websocket connection and the hub.
type SocketClient struct {
	ClientData
	conn    *websocket.Conn
	send    chan Outbound
	once    sync.Once
	ip      net.IP
	counter int // counts up every send
}

// Create a SocketClient from a connection
func NewSocketClient(conn *websocket.Conn, ip net.IP) *SocketClient {
	return &SocketClient{
		conn: conn,
		ip:   ip,
		send: make(chan Outbound, socketBufferSize),
	}
}

func (client *SocketClient) Bot() bool {
	return false
}

func (client *SocketClient) Close() {
	close(client.send)
}

func (client *SocketClient) Data() *ClientData {
	return &client.ClientData
}

func (client *SocketClient) Destroy() {
	client.once.Do(func() {
		client.Hub.Unregister(client)

		_ = client.conn.Close()

		if client.ip != nil {
			client.Hub.ipMu.Lock()
			defer client.Hub.ipMu.Unlock()
			str := client.ip.String()
			client.Hub.ipConns[str]--
			if client.Hub.ipConns[str] <= 0 {
				delete(client.Hub.ipConns, str)
			}
		}
	})
}

func (client *SocketClient) Init() {
	go client.writePump()
	go client.readPump()
}

func (client *SocketClient) Send(message Outbound) {
	// How many messages there are in excess of a reasonable amount
	congestion := len(client.send) - socketCongestionThreshold

	// The closer the buffer is to being full, the more messages
	// we drop on the floor (to give the socket a chance to
	// catch up)
	client.counter++
	if congestion > 1 && client.counter%congestion != 0 {
		// Drop the message on the floor
		// The only long-term data loss will be from event-based things
		// like chat messages
		fmt.Println("SocketClient dropping message due to congestion")
		return
	}

	select {
	case client.send <- message:
	default:
		// Not responsive
		if debugSocket {
			fmt.Println("SocketClient is not responsive")
		}
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
			client.Hub.ReceiveSigned(SignedInbound{Client: client, Inbound: message.Data.(Inbound)}, true)
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
				panic("hub closed channel")
			}

			w, err := client.conn.NextWriter(websocket.TextMessage)
			if err != nil {
				panic(err)
			}

			// Wrap with Message to marshal type
			if err = json.NewEncoder(w).Encode(Message{Data: out}); err != nil {
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
