// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	_ "embed"
	"encoding/json"
	"errors"
	"sort"
)

const (
	EntityKindInvalid    = EntityKind(invalidEnumChoice)
	EntitySubKindInvalid = EntitySubKind(invalidEnumChoice)
	EntityTypeInvalid    = EntityType(invalidEnumChoice)
	SensorTypeInvalid    = SensorType(invalidEnumChoice)
	invalidEnumChoice    = 0
)

var (
	entityKindEnum    enum
	entitySubKindEnum enum
	entityTypeData    []EntityTypeData
	entityTypeEnum    enum
	sensorTypeEnum    enum
)

type (
	// entityTypeLoader loads an EntityType before the enums are defined
	entityTypeLoader struct {
		Kind    string         `json:"type"`
		SubKind string         `json:"subtype"`
		Sensors []sensorLoader `json:"sensors"`
	}

	sensorLoader struct {
		Type string `json:"type"`
	}

	// enum is a list of possible choices and their strings
	enum struct {
		choices map[string]enumChoice // choices maps from strings to choices
		strings []string              // strings maps from choices to strings
		name    string                // name of enum for error
	}

	// enumChoice is a choice of an enum
	// Only use uint8 because only 255 options are needed (plus invalid)
	enumChoice uint8
)

func (enum *enum) add(s string) {
	if enum.strings == nil {
		enum.strings = []string{"invalid"}
	}

	// Check uniqueness
	for _, other := range enum.strings {
		if s == other {
			return
		}
	}

	enum.strings = append(enum.strings, s)
}

func (enum *enum) create(name string) {
	// Sort strings but invalid must remain at index 0
	sort.Strings(enum.strings[invalidEnumChoice+1:])

	enum.choices = make(map[string]enumChoice, len(enum.strings)-1)
	for i, s := range enum.strings {
		// Skip invalid
		if i == invalidEnumChoice {
			continue
		}
		enum.choices[s] = enumChoice(i)
	}

	enum.name = name
}

func (enum *enum) mustParse(s string) enumChoice {
	c, ok := enum.choices[s]
	if !ok {
		panic("invalid " + enum.name + ": " + s)
	}
	return c
}

func (c *enumChoice) unmarshalText(enum *enum, text []byte) error {
	var ok bool
	*c, ok = enum.choices[string(text)]
	if !ok {
		return errors.New("invalid " + enum.name + ": " + string(text))
	}
	return nil
}

//go:embed entities.json
var entityDataJSON []byte

func init() {
	typeLoaders := make(map[string]entityTypeLoader)
	err := json.Unmarshal(entityDataJSON, &typeLoaders)
	if err != nil {
		panic(err)
	}

	for t, d := range typeLoaders {
		entityKindEnum.add(d.Kind)
		entitySubKindEnum.add(d.SubKind)
		entityTypeEnum.add(t)
		for _, s := range d.Sensors {
			sensorTypeEnum.add(s.Type)
		}
	}

	entityKindEnum.create("entity kind")
	entitySubKindEnum.create("entity sub kind")
	entityTypeEnum.create("entity type")
	sensorTypeEnum.create("sensor type")

	// Unmarshal data now that enums are created
	entityData := make(map[string]EntityTypeData)
	err = json.Unmarshal(entityDataJSON, &entityData)
	if err != nil {
		panic(err)
	}

	// EntityType to EntityTypeData
	entityTypeData = make([]EntityTypeData, len(entityTypeEnum.strings))
	for i, s := range entityTypeEnum.strings {
		// Skip invalid
		if i == invalidEnumChoice {
			continue
		}

		data := &entityTypeData[i]
		*data = entityData[s]

		data.Radius = Vec2f{X: data.Width, Y: data.Length}.Mul(0.5).Length()

		EntityRadiusMax = max(data.Radius, EntityRadiusMax)
		if data.Level > EntityLevelMax {
			EntityLevelMax = data.Level
		}

		data.InvSize = 1.0 / min(1, data.Radius*(1.0/30.0)*(1.0-data.Stealth))
	}

	EntityKindBoat = ParseEntityKind("boat")
	EntityKindCollectible = ParseEntityKind("collectible")
	EntityKindDecoy = ParseEntityKind("decoy")
	EntityKindObstacle = ParseEntityKind("obstacle")
	EntityKindWeapon = ParseEntityKind("weapon")

	EntitySubKindAircraft = ParseEntitySubKind("aircraft")
	EntitySubKindBattleship = ParseEntitySubKind("battleship")
	EntitySubKindDepositor = ParseEntitySubKind("depositor")
	EntitySubKindDepthCharge = ParseEntitySubKind("depthCharge")
	EntitySubKindDredger = ParseEntitySubKind("dredger")
	EntitySubKindHovercraft = ParseEntitySubKind("hovercraft")
	EntitySubKindMine = ParseEntitySubKind("mine")
	EntitySubKindMissile = ParseEntitySubKind("missile")
	EntitySubKindPirate = ParseEntitySubKind("pirate")
	EntitySubKindTorpedo = ParseEntitySubKind("torpedo")
	EntitySubKindRam = ParseEntitySubKind("ram")
	EntitySubKindRocket = ParseEntitySubKind("rocket")
	EntitySubKindSAM = ParseEntitySubKind("sam")
	EntitySubKindShell = ParseEntitySubKind("shell")
	EntitySubKindSonar = ParseEntitySubKind("sonar")
	EntitySubKindSubmarine = ParseEntitySubKind("submarine")

	EntityTypeBarrel = ParseEntityType("barrel")
	EntityTypeCoin = ParseEntityType("coin")
	EntityTypeCount = len(entityTypeEnum.strings)
	EntityTypeCrate = ParseEntityType("crate")
	EntityTypeMark18 = ParseEntityType("mark18")
	EntityTypeOilPlatform = ParseEntityType("oilPlatform")
	EntityTypeScrap = ParseEntityType("scrap")

	SensorTypeRadar = ParseSensorType("radar")
	SensorTypeSonar = ParseSensorType("sonar")
	SensorTypeVisual = ParseSensorType("visual")

	// Spawn entities are boats that are level 1
	for i, data := range entityTypeData {
		if data.Kind == EntityKindBoat {
			for len(BoatEntityTypesByLevel) <= int(data.Level) {
				BoatEntityTypesByLevel = append(BoatEntityTypesByLevel, []EntityType{})
			}
			BoatEntityTypesByLevel[data.Level] = append(BoatEntityTypesByLevel[data.Level], EntityType(i))
		}
	}
}

// Enums used in code

var (
	EntityKindBoat        EntityKind
	EntityKindCollectible EntityKind
	EntityKindDecoy       EntityKind
	EntityKindObstacle    EntityKind
	EntityKindWeapon      EntityKind

	EntitySubKindAircraft    EntitySubKind
	EntitySubKindBattleship  EntitySubKind
	EntitySubKindDepositor   EntitySubKind
	EntitySubKindDepthCharge EntitySubKind
	EntitySubKindDredger     EntitySubKind
	EntitySubKindHovercraft  EntitySubKind
	EntitySubKindPirate      EntitySubKind
	EntitySubKindMine        EntitySubKind
	EntitySubKindMissile     EntitySubKind
	EntitySubKindRam         EntitySubKind
	EntitySubKindRocket      EntitySubKind
	EntitySubKindSAM         EntitySubKind
	EntitySubKindShell       EntitySubKind
	EntitySubKindSonar       EntitySubKind
	EntitySubKindSubmarine   EntitySubKind
	EntitySubKindTorpedo     EntitySubKind

	EntityTypeBarrel      EntityType
	EntityTypeCoin        EntityType
	EntityTypeCrate       EntityType
	EntityTypeMark18      EntityType
	EntityTypeOilPlatform EntityType
	EntityTypeScrap       EntityType

	SensorTypeRadar  SensorType
	SensorTypeSonar  SensorType
	SensorTypeVisual SensorType
)

// EntityKind helpers

func (entityKind EntityKind) AppendText(buf []byte) []byte {
	return append(buf, entityKind.String()...)
}

func (entityKind EntityKind) MarshalText() ([]byte, error) {
	return entityKind.AppendText(nil), nil
}

func ParseEntityKind(s string) EntityKind {
	return EntityKind(entityKindEnum.mustParse(s))
}

func (entityKind EntityKind) String() string {
	return entityKindEnum.strings[entityKind]
}

func (entityKind *EntityKind) UnmarshalText(text []byte) (err error) {
	var choice enumChoice
	err = choice.unmarshalText(&entityKindEnum, text)
	*entityKind = EntityKind(choice)
	return
}

// EntitySubKind helpers

func (entitySubKind EntitySubKind) AppendText(buf []byte) []byte {
	return append(buf, entitySubKind.String()...)
}

func (entitySubKind EntitySubKind) MarshalText() ([]byte, error) {
	return entitySubKind.AppendText(nil), nil
}

func ParseEntitySubKind(s string) EntitySubKind {
	return EntitySubKind(entitySubKindEnum.mustParse(s))
}

func (entitySubKind EntitySubKind) String() string {
	return entitySubKindEnum.strings[entitySubKind]
}

func (entitySubKind *EntitySubKind) UnmarshalText(text []byte) (err error) {
	var choice enumChoice
	err = choice.unmarshalText(&entitySubKindEnum, text)
	*entitySubKind = EntitySubKind(choice)
	return
}

// EntityType helpers

func (entityType EntityType) AppendText(buf []byte) []byte {
	return append(buf, entityType.String()...)
}

func (entityType EntityType) MarshalText() ([]byte, error) {
	return entityType.AppendText(nil), nil
}

func ParseEntityType(s string) EntityType {
	return EntityType(entityTypeEnum.mustParse(s))
}

func (entityType EntityType) String() string {
	return entityTypeEnum.strings[entityType]
}

func (entityType *EntityType) UnmarshalText(text []byte) (err error) {
	var choice enumChoice
	err = choice.unmarshalText(&entityTypeEnum, text)
	*entityType = EntityType(choice)
	return
}

// SensorType helpers

func (sensorType SensorType) AppendText(buf []byte) []byte {
	return append(buf, sensorType.String()...)
}

func (sensorType SensorType) MarshalText() ([]byte, error) {
	return sensorType.AppendText(nil), nil
}

func ParseSensorType(s string) SensorType {
	return SensorType(sensorTypeEnum.mustParse(s))
}

func (sensorType SensorType) String() string {
	return sensorTypeEnum.strings[sensorType]
}

func (sensorType *SensorType) UnmarshalText(text []byte) (err error) {
	var choice enumChoice
	err = choice.unmarshalText(&sensorTypeEnum, text)
	*sensorType = SensorType(choice)
	return
}
