// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

import (
	"errors"
	"github.com/SoftbearStudios/mk48/server/world"
	jsoniter "github.com/json-iterator/go"
	"reflect"
	"sort"
	"sync"
	"unsafe"
)

// Make sure functions get run first
var json = func() jsoniter.API {
	neverEmpty := func(pointer unsafe.Pointer) bool { return false }

	// Encoders
	jsoniter.RegisterFieldEncoderFunc(reflect.TypeOf(Update{}).String(), "Contacts", encodeUpdateContacts, neverEmpty)
	jsoniter.RegisterTypeEncoderFunc(reflect.TypeOf(world.EntityID(0)).String(), encodeEntityID, emptyEntityID)
	jsoniter.RegisterTypeEncoderFunc(reflect.TypeOf(world.EntityType(0)).String(), encodeEntityType, neverEmpty)
	jsoniter.RegisterTypeEncoderFunc(reflect.TypeOf(Message{}).String(), encodeMessage, neverEmpty)
	jsoniter.RegisterTypeEncoderFunc(reflect.TypeOf(world.PlayerID(0)).String(), encodePlayerID, emptyPlayerID)
	jsoniter.RegisterTypeEncoderFunc(reflect.TypeOf(world.TeamID(0)).String(), encodeTeamID, emptyTeamID)
	jsoniter.RegisterTypeEncoderFunc(reflect.TypeOf(world.Angle(0)).String(), encodeAngle, emptyAngle)
	jsoniter.RegisterTypeEncoderFunc(reflect.TypeOf(world.Velocity(0)).String(), encodeVelocity, emptyVelocity)
	jsoniter.RegisterTypeEncoderFunc(reflect.TypeOf(world.Ticks(0)).String(), encodeTicks, emptyTicks)

	// Decoders
	jsoniter.RegisterTypeDecoderFunc(reflect.TypeOf(Message{}).String(), decodeMessage)
	jsoniter.RegisterTypeDecoderFunc(reflect.TypeOf(world.Angle(0)).String(), decodeAngle)
	jsoniter.RegisterTypeDecoderFunc(reflect.TypeOf(world.Velocity(0)).String(), decodeVelocity)
	jsoniter.RegisterTypeDecoderFunc(reflect.TypeOf(world.Ticks(0)).String(), decodeTicks)

	return jsoniter.Config{
		IndentionStep:                 0,
		MarshalFloatWith6Digits:       true,
		EscapeHTML:                    false,
		SortMapKeys:                   true,
		UseNumber:                     false,
		DisallowUnknownFields:         false,
		TagKey:                        "json",
		OnlyTaggedField:               false,
		ValidateJsonRawMessage:        false,
		ObjectFieldMustBeSimpleString: true,
		CaseSensitive:                 true,
	}.Froze()
}()

func encodeMessage(ptr unsafe.Pointer, stream *jsoniter.Stream) {
	message := (*Message)(ptr)
	stream.WriteVal(message.messageJSON())
}

var sortedContactsPool = sync.Pool{
	New: func() interface{} {
		slice := make([]*IDContact, 0, poolContactsCap)
		return &slice
	},
}

// Encodes Update.Contacts as a map in json
func encodeUpdateContacts(ptr unsafe.Pointer, stream *jsoniter.Stream) {
	contacts := *(*[]IDContact)(ptr)

	// Reallocate to slice of pointers for faster swaps (~40% faster not counting extra gc)
	sortedContactsPtr := sortedContactsPool.Get().(*[]*IDContact)
	sortedContacts := *sortedContactsPtr

	for i := range contacts {
		sortedContacts = append(sortedContacts, &contacts[i])
	}

	sort.Slice(sortedContacts, func(i, j int) bool {
		return sortedContacts[i].EntityID < sortedContacts[j].EntityID
	})

	stream.WriteObjectStart()
	first := true
	for _, c := range sortedContacts {
		// Quote
		if first {
			first = false
		} else {
			stream.WriteMore()
		}

		// Flush stream because buffer is 512 bytes and average contact is around 300 bytes
		if stream.Error != nil {
			return
		}
		_ = stream.Flush()

		// Map key of EntityID quoted
		stream.SetBuffer(append(c.EntityID.AppendText(append(stream.Buffer(), '"')), '"', ':'))

		// Map value of Contact
		stream.WriteVal(&c.Contact)
	}
	stream.WriteObjectEnd()

	// Clear pointers
	for i := range sortedContacts {
		sortedContacts[i] = nil
	}

	// Pool sorted contacts with pointer to slice as to not allocate slice header
	*sortedContactsPtr = sortedContacts[:0]
	sortedContactsPool.Put(sortedContactsPtr)
}

func encodeAngle(ptr unsafe.Pointer, stream *jsoniter.Stream) {
	angle := *(*world.Angle)(ptr)
	stream.WriteFloat32Lossy(angle.Float())
}

func emptyAngle(ptr unsafe.Pointer) bool {
	return *(*world.Angle)(ptr) == 0
}

func encodeEntityID(ptr unsafe.Pointer, stream *jsoniter.Stream) {
	id := *(*world.EntityID)(ptr)
	// Quoted hex
	stream.SetBuffer(append(id.AppendText(append(stream.Buffer(), '"')), '"'))
}

func emptyEntityID(ptr unsafe.Pointer) bool {
	return *(*world.EntityID)(ptr) == world.EntityIDInvalid
}

func encodeEntityType(ptr unsafe.Pointer, stream *jsoniter.Stream) {
	entityType := *(*world.EntityType)(ptr)
	// Quoted hex
	stream.SetBuffer(append(entityType.AppendText(append(stream.Buffer(), '"')), '"'))
}

func encodePlayerID(ptr unsafe.Pointer, stream *jsoniter.Stream) {
	id := *(*world.PlayerID)(ptr)
	// Quoted hex
	stream.SetBuffer(append(id.AppendText(append(stream.Buffer(), '"')), '"'))
}

func emptyPlayerID(ptr unsafe.Pointer) bool {
	return *(*world.PlayerID)(ptr) == world.PlayerIDInvalid
}

func encodeTicks(ptr unsafe.Pointer, stream *jsoniter.Stream) {
	ticks := *(*world.Ticks)(ptr)
	stream.WriteFloat32Lossy(ticks.Float())
}

func emptyTicks(ptr unsafe.Pointer) bool {
	return *(*world.Ticks)(ptr) == 0
}

func encodeTeamID(ptr unsafe.Pointer, stream *jsoniter.Stream) {
	id := *(*world.TeamID)(ptr)
	// Short string
	stream.SetBuffer(append(id.AppendText(append(stream.Buffer(), '"')), '"'))
}

func emptyTeamID(ptr unsafe.Pointer) bool {
	return *(*world.TeamID)(ptr) == world.TeamIDInvalid
}

func encodeVelocity(ptr unsafe.Pointer, stream *jsoniter.Stream) {
	velocity := *(*world.Velocity)(ptr)
	stream.WriteFloat32Lossy(velocity.Float())
}

func emptyVelocity(ptr unsafe.Pointer) bool {
	return *(*world.Velocity)(ptr) == 0
}

func decodeAngle(ptr unsafe.Pointer, iter *jsoniter.Iterator) {
	f := iter.ReadFloat32()
	*(*world.Angle)(ptr) = world.ToAngle(f)
}

func decodeTicks(ptr unsafe.Pointer, iter *jsoniter.Iterator) {
	f := iter.ReadFloat32()
	*(*world.Ticks)(ptr) = world.ToTicks(f)
}

func decodeVelocity(ptr unsafe.Pointer, iter *jsoniter.Iterator) {
	f := iter.ReadFloat32()
	*(*world.Velocity)(ptr) = world.ToVelocity(f)
}

// Buffers large enough to hold most inbounds
var decodeMessagePool = sync.Pool{
	New: func() interface{} {
		buf := make([]byte, 0, 256)
		return &buf
	},
}

func decodeMessage(ptr unsafe.Pointer, topLevelIter *jsoniter.Iterator) {
	bufPtr := decodeMessagePool.Get().(*[]byte)

	// Read bytes so can read twice
	messageBytes := topLevelIter.SkipAndAppendBytes(*bufPtr)

	// Pool iterator with previous pool
	pool := topLevelIter.Pool()
	iter := pool.BorrowIterator(messageBytes)
	defer pool.ReturnIterator(iter)

	// Interface of *inbound
	var in interface{}

	// Doesn't have to read twice if type is first field
	// If type is found c is > 0
	for c := 0; c < 3; c++ {
		iter.ResetBytes(messageBytes)
		iter.ReadObjectCB(func(i *jsoniter.Iterator, field string) bool {
			if field == "type" {
				// Not already read
				if in == nil {
					messageTypeBytes := i.ReadStringAsSlice()
					inboundType, ok := inboundMessageTypes[messageType(messageTypeBytes)]
					if !ok {
						inboundType = reflect.TypeOf(InvalidInbound{})
					}
					in = reflect.New(inboundType).Interface()

					if !ok {
						in.(*InvalidInbound).messageType = messageType(messageTypeBytes)
					}

					c++
				} else {
					i.Skip()
				}
				return true
			} else if field == "data" {
				// Found type
				if c > 0 {
					i.ReadVal(in)
					c++
					return false // Finished
				} else {
					i.Skip()
				}
			} else {
				i.Skip()
			}
			return true
		})

		if err := iter.Error; err != nil {
			topLevelIter.Error = err
			return
		}

		// No message type
		if c == 0 {
			topLevelIter.Error = errors.New("no inbound message type")
			return
		}
	}

	// Pool messageBytes
	*bufPtr = messageBytes[:0]
	decodeMessagePool.Put(bufPtr)

	// Store data
	message := (*Message)(ptr)
	message.Data = reflect.Indirect(reflect.ValueOf(in)).Interface()
}
