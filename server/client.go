// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

type (
	// Client is an actor on the Hub.
	Client interface {
		// Init is called once called by hub goroutine when the client is registered.
		// client.Data().Hub will be set by the time this is called
		Init()

		// Close is called by (only) the hub goroutine when the client is unregistered.
		Close()

		// Send is how the server sends a message to the client.
		Send(out outbound)

		// Destroy marks the client for destruction. It must call hub.Unregister() only once (no matter how many
		// times it is called; use a sync.Once if necessary). It may be called anywhere.
		Destroy()

		// Is this a bot or real player?
		Bot() bool

		// Data allows the Client to be added to a double-linked list.
		Data() *ClientData
	}

	// ClientData is the data all clients must have.
	ClientData struct {
		Player   Player
		Hub      *Hub
		Previous Client
		Next     Client
	}

	// ClientList is a doubly-linked list of Clients.
	// It can be iterated like this:
	// for client := list.First; client != nil; client = client.Data().Next {}
	// Or to remove all iterated items like this:
	// for client := list.First; client != nil; client = list.Remove(client) {}
	ClientList struct {
		First Client
		Last  Client
		Len   int
	}
)

// Add adds a Client to the list.
func (list *ClientList) Add(client Client) {
	data := client.Data()
	if data.Previous != nil || data.Next != nil {
		panic("already added")
	}

	// Repair list
	if list.First == nil {
		list.First = client
	} else if list.Last == nil {
		panic("invalid state")
	} else {
		list.Last.Data().Next = client
		data.Previous = list.Last
	}

	list.Last = client
	list.Len++
}

// Remove removes a Client from the list.
// Returns the next element of the list.
func (list *ClientList) Remove(client Client) (next Client) {
	data := client.Data()

	// Repair list
	if data.Previous != nil {
		data.Previous.Data().Next = data.Next
	} else if list.First == client {
		list.First = data.Next
	} else {
		panic("already removed")
	}

	if data.Next != nil {
		data.Next.Data().Previous = data.Previous
	} else if list.Last == client {
		list.Last = data.Previous
	} else {
		panic("already removed")
	}

	list.Len--
	next = data.Next
	data.Next = nil
	data.Previous = nil

	return
}
