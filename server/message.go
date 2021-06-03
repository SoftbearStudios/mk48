// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

import (
	"reflect"
	"strings"
)

var (
	// Valid inbound message types: messageType to type
	inboundMessageTypes = make(map[messageType]reflect.Type)
	// Valid outbound message types: to messageType
	outboundMessageTypes = make(map[reflect.Type]messageType)
)

type (
	inbound interface {
		Inbound(hub *Hub, client Client, player *Player)
	}

	outbound interface {
		// Pool returns the contents of outbound to their sync.Pool
		Pool()
	}

	Message struct {
		Data interface{}
	}

	messageJSON struct {
		Data interface{} `json:"data"`
		Type messageType `json:"type"`
	}

	messageType string

	SignedInbound struct {
		Client Client
		inbound
	}
)

func uncapitalize(str string) string {
	return strings.ToLower(str[0:1]) + str[1:]
}

func registerInbound(inbounds ...inbound) {
	for _, in := range inbounds {
		val := reflect.ValueOf(in)
		m := messageType(uncapitalize(reflect.Indirect(val).Type().Name()))
		inboundMessageTypes[m] = val.Type()
	}
}

func registerOutbound(outbounds ...outbound) {
	for _, out := range outbounds {
		val := reflect.ValueOf(out)
		m := messageType(uncapitalize(reflect.Indirect(val).Type().Name()))
		outboundMessageTypes[val.Type()] = m
	}
}

func (message Message) messageJSON() messageJSON {
	typ := reflect.TypeOf(message.Data)

	// Outbounds are marshaled
	mType, ok := outboundMessageTypes[typ]
	if !ok {
		// Panic because outbounds only come from trusted sources
		panic("invalid outbound message type " + typ.Name())
	}

	return messageJSON{Data: message.Data, Type: mType}
}

// Overridden by jsoniter
func (message Message) MarshalJSON() ([]byte, error) {
	panic("unimplemented")
}

// Overridden by jsoniter
func (message *Message) UnmarshalJSON([]byte) error {
	panic("unimplemented")
}
