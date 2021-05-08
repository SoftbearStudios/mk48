// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"reflect"
	"testing"
)

var testingPlayers [4]Player

func testPtr(t *testing.T, a, b interface{}) {
	if a != b {
		t.Fatalf("expected %p got %p", b, a)
	}
}

func TestPlayerSet_Add(t *testing.T) {
	set := PlayerSet{}
	set.Add(&testingPlayers[0])
	set.Add(&testingPlayers[1])

	if len(set) != 2 {
		t.Fatalf("expected length %v got %v", 2, len(set))
	}

	// Already in set
	set.Add(&testingPlayers[0])

	if len(set) != 2 {
		t.Fatalf("expected length %v got %v", 2, len(set))
	}

	// Not in set
	set.Add(&testingPlayers[2])

	if len(set) != 3 {
		t.Fatalf("expected length %v got %v", 3, len(set))
	}
}

func TestPlayerSet_Remove(t *testing.T) {
	set := PlayerSet{}
	for i := range testingPlayers {
		set.Add(&testingPlayers[i])
	}

	if len(set) != len(testingPlayers) {
		t.Fatalf("expected length %v got %v", len(testingPlayers), len(set))
	}

	removeIndex := 2
	set.Remove(&testingPlayers[removeIndex])

	if len(set) != len(testingPlayers)-1 {
		t.Fatalf("expected length %v got %v", len(testingPlayers)-1, len(set))
	}

	set2 := PlayerSet{}
	for i := range testingPlayers {
		if i == removeIndex {
			continue
		}
		set2.Add(&testingPlayers[i])
	}

	if !reflect.DeepEqual(set2, set) {
		t.Fatalf("expected %v got %v", set2, set)
	}
}

func TestPlayerSet_GetByID(t *testing.T) {
	set := PlayerSet{}

	// Never in set
	testPtr(t, set.GetByID(PlayerIDInvalid), (*Player)(nil))

	// Not in set
	for i := range testingPlayers {
		p := set.GetByID(testingPlayers[i].PlayerID())
		e := (*Player)(nil)
		if p != e {
			t.Fatalf("expected %p got %p", e, p)
		}
	}

	// Add
	for i := range testingPlayers {
		set.Add(&testingPlayers[i])
	}

	// In set
	for i := range testingPlayers {
		p := set.GetByID(testingPlayers[i].PlayerID())
		e := &testingPlayers[i]
		if p != e {
			t.Fatalf("expected %p got %p", e, p)
		}
	}

	removeIndex := 0
	set.Remove(&testingPlayers[removeIndex])

	p := set.GetByID(testingPlayers[removeIndex].PlayerID())
	e := (*Player)(nil)
	if p != e {
		t.Fatalf("expected %p got %p", e, p)
	}

	// Never in set
	testPtr(t, set.GetByID(PlayerIDInvalid), (*Player)(nil))
}
