// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"github.com/SoftbearStudios/mk48/server"
	"log"
	"syscall/js"
)

var (
	self        = js.Global().Get("self")
	postMessage = self.Get("postMessage")
	localClient LocalClient // only one of these ever exists
)

func init() {
	self.Set("onmessage", js.FuncOf(func(this js.Value, args []js.Value) interface{} {
		data := []byte(args[0].Get("data").String())

		var message server.Message
		err := server.JSON.Unmarshal(data, &message)
		if err != nil {
			log.Println("unmarshal error:", err.Error())
			return nil
		}

		if _, ok := message.Data.(server.InvalidInbound); ok {
			log.Println("invalid message type received")
		} else {
			localClient.Hub.ReceiveSigned(server.SignedInbound{Client: &localClient, Inbound: message.Data.(server.Inbound)}, true)
		}

		return nil
	}))
}

// There may only ever be one local client
type LocalClient struct {
	server.ClientData
}

func (client *LocalClient) Bot() bool {
	return false
}

func (client *LocalClient) Close() {
	panic("local client closed")
}

func (client *LocalClient) Data() *server.ClientData {
	return &client.ClientData
}

func (client *LocalClient) Destroy() {
	panic("local client destroyed")
}

func (client *LocalClient) Init() {}

func (client *LocalClient) Send(out server.Outbound) {
	buf, err := server.JSON.Marshal(server.Message{Data: out})
	if err != nil {
		panic(err)
	}

	postMessage.Invoke(string(buf))

	out.Pool()
}
