// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

import (
	"reflect"
	"strings"
)

var (
	// Valid Inbound message types: messageType to type
	inboundMessageTypes = make(map[messageType]reflect.Type)
	// Valid Outbound message types: to messageType
	outboundMessageTypes = make(map[reflect.Type]messageType)
)

type (
	Inbound interface {
		Process(hub *Hub, client Client, player *Player)
	}

	Outbound interface {
		// Pool returns the contents of Outbound to their sync.Pool
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
		Inbound
	}
)

func uncapitalize(str string) string {
	return strings.ToLower(str[0:1]) + str[1:]
}

func registerInbound(inbounds ...Inbound) {
	for _, in := range inbounds {
		val := reflect.ValueOf(in)
		m := messageType(uncapitalize(reflect.Indirect(val).Type().Name()))
		inboundMessageTypes[m] = val.Type()
	}
}

func registerOutbound(outbounds ...Outbound) {
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
		panic("invalid Outbound message type " + typ.Name())
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
