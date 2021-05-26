// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"strings"
	"unicode"
)

var (
	EntityLevelMax         uint8
	EntityRadiusMax        float32
	EntityTypeCount        int
	BoatEntityTypesByLevel [][]EntityType
)

type (
	// Armament is the description of an armament in an EntityData.
	// Armaments are not only weapons but also countermeasures.
	Armament struct {
		Type            EntityKind    `json:"type"`
		Subtype         EntitySubKind `json:"subtype"`
		Default         EntityType    `json:"default"`
		Airdrop         bool          `json:"airdrop"`
		Vertical        bool          `json:"vertical"`
		Length          float32       `json:"length"`
		Width           float32       `json:"width"`
		PositionForward float32       `json:"positionForward"`
		PositionSide    float32       `json:"positionSide"`
		Angle           Angle         `json:"angle"`
		Turret          *int          `json:"turret,omitempty"` // If non-nil, index of turret
	}

	// EntityKind is the kind of an entity one of: boat, collectible, obstacle, and weapon.
	EntityKind enumChoice

	// EntitySubKind is the sub-kind of an entity such as: depth charge, missile, MTB, score, shell, submarine, torpedo, etc.
	EntitySubKind enumChoice

	// EntityType is the type of an entity such as: crate, fairmileD, etc.
	EntityType enumChoice

	// EntityTypeData is the description of an EntityType.
	EntityTypeData struct {
		// All units are SI (meters, seconds, etc.)
		Kind         EntityKind    `json:"type"`
		SubKind      EntitySubKind `json:"subtype"`
		Level        uint8         `json:"level"`
		Limited      bool          `json:"limited"`
		NPC          bool          `json:"npc"` // only bots can use
		Lifespan     Ticks         `json:"lifespan"`
		Reload       Ticks         `json:"reload"` // time to reload
		Speed        Velocity      `json:"speed"`
		Length       float32       `json:"length"`
		Width        float32       `json:"width"`
		Radius       float32       `json:"-"`
		InvSize      float32       `json:"-"`
		Damage       float32       `json:"damage"`
		AntiAircraft float32       `json:"antiAircraft"` // chance aircraft is shot down per second
		Stealth      float32       `json:"stealth"`
		Sensors      []Sensor      `json:"sensors"`
		Armaments    []Armament    `json:"armaments"`
		Turrets      []Turret      `json:"turrets"`
		Label        string        `json:"label"`
	}

	// Sensor the description of a sensor in an EntityType.
	Sensor struct {
		Type  SensorType `json:"type"`
		Range float32    `json:"range"`
	}

	// SensorType is the type of a sensor one of: visual, radar, and sonar.
	SensorType enumChoice

	// Turret is the description of a turret's relative transform in an EntityType.
	Turret struct {
		PositionForward float32 `json:"positionForward"`
		PositionSide    float32 `json:"positionSide"`
		Angle           Angle   `json:"angle"`
	}
)

// TurretIndex returns the index of the turret if it exists else -1.
func (armament *Armament) TurretIndex() int {
	if t := armament.Turret; t != nil {
		return *t
	}
	return -1
}

// Reload returns the time it takes to reload an Armament in seconds.
func (armament *Armament) Reload() Ticks {
	consumption := armament.Default.Data().Reload
	if armament.Airdrop {
		// Maybe should check for overflow.
		consumption *= 4
	}
	return consumption
}

// Similar returns if the Armament is on the same turret and the same type as other.
func (armament *Armament) Similar(other *Armament) bool {
	return armament.Default == other.Default && armament.TurretIndex() == other.TurretIndex()
}

func (entityType EntityType) Data() *EntityTypeData {
	return &entityTypeData[entityType]
}

// MaxHealth is the maximum health of an EntityType as an absolute
func (entityType EntityType) MaxHealth() float32 {
	return max(1, entityType.Data().Length*(1.0/32.0))
}

// Returns a lifespan to start an entity's life at, so as to make it expire
// in desiredLifespan ticks
func (entityType EntityType) ReducedLifespan(desiredLifespan Ticks) Ticks {
	data := entityType.Data()
	if data.Lifespan > desiredLifespan {
		return data.Lifespan - desiredLifespan
	}
	return data.Lifespan
}

func (entityType EntityType) UpgradePaths(score int) (upgradePaths []EntityType) {
	if levelToScore(entityType.Data().Level+1) > score {
		return
	}

	for i := range entityTypeData {
		nextEntityType := EntityType(i)
		if entityType.UpgradesTo(nextEntityType, score) {
			if upgradePaths == nil {
				upgradePaths = make([]EntityType, 0, 8)
			}
			upgradePaths = append(upgradePaths, nextEntityType)
		}
	}
	return
}

func (entityType EntityType) UpgradesTo(nextEntityType EntityType, score int) bool {
	data := entityType.Data()
	nextData := nextEntityType.Data()

	return nextData.Level > data.Level && nextData.Kind == data.Kind && score >= levelToScore(nextEntityType.Data().Level)
}

// depthCharge -> depth charge
func (entitySubKind EntitySubKind) Label() string {
	str := entitySubKind.String()
	var builder strings.Builder
	for _, r := range str {
		if unicode.IsUpper(r) {
			builder.WriteByte(' ')
		}
		builder.WriteRune(unicode.ToLower(r))
	}
	return builder.String()
}

// levelToScore converts a boat level to a score required to upgrade
// score = (level^2 - 1) * 10
// js must have same function
func levelToScore(level uint8) int {
	l := int(level)
	return (l*l - 1) * 10
}
