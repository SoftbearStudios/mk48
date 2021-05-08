// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"errors"
	"math/rand"
	"strconv"
)

const EntityIDInvalid = EntityID(0)

type EntityID uint32

func AllocateEntityID(used func(id EntityID) bool) (uniqueID EntityID) {
	for i := 0; i < 10; i++ {
		// Use shorter EntityIDs first to save on json
		chars := i + 1
		if chars > 8 {
			chars = 8
		}

		uniqueID = EntityID(rand.Intn(1 << (chars * 4)))
		if uniqueID == EntityIDInvalid {
			continue
		}

		if !used(uniqueID) {
			return uniqueID
		}
	}
	panic("could not find unique EntityID in 10 tries")
}

func (entityID EntityID) String() string {
	buf, err := entityID.MarshalText()
	if err != nil {
		return "invalid"
	}
	return string(buf)
}

func (entityID EntityID) MarshalText() ([]byte, error) {
	return entityID.AppendText(make([]byte, 0, 8)), nil
}

func (entityID EntityID) AppendText(buf []byte) []byte {
	if entityID == EntityIDInvalid {
		panic("invalid entity id")
	}
	return strconv.AppendUint(buf, uint64(entityID), 16)
}

var entityIDInvalidErr = errors.New("invalid entity id")

func (entityID *EntityID) UnmarshalText(text []byte) error {
	i, err := strconv.ParseUint(string(text), 16, 32)
	*entityID = EntityID(i)
	if err == nil && *entityID == EntityIDInvalid {
		err = entityIDInvalidErr
	}
	return err
}
