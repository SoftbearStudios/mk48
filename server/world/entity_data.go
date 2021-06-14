// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"github.com/chewxy/math32"
)

var (
	BoatLevelMax           uint8
	EntityRadiusMax        float32
	EntityTypeCount        int
	BoatEntityTypesByLevel [][]EntityType
)

type (
	// Armament is the description of an armament in an EntityData.
	// Armaments are not only weapons but also countermeasures.
	Armament struct {
		Type            EntityType `json:"type"`
		Vertical        bool       `json:"vertical"`
		Length          float32    `json:"length"`
		Width           float32    `json:"width"`
		PositionForward float32    `json:"positionForward"`
		PositionSide    float32    `json:"positionSide"`
		Angle           Angle      `json:"angle"`
		Turret          *int       `json:"turret,omitempty"` // If non-nil, index of turret
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
		Kind         EntityKind    `json:"kind"`
		SubKind      EntitySubKind `json:"subkind"`
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
		Damage       float32       `json:"damage"`       // health of ship, or damage dealt by weapon
		AntiAircraft float32       `json:"antiAircraft"` // chance aircraft is shot down per second
		Stealth      float32       `json:"stealth"`
		Sensors      Sensors       `json:"sensors"`
		Armaments    []Armament    `json:"armaments"`
		Turrets      []Turret      `json:"turrets"`
		Label        string        `json:"label"`
	}

	Sensors struct {
		Visual Sensor `json:"visual"`
		Radar  Sensor `json:"radar"`
		Sonar  Sensor `json:"sonar"`
	}

	// Sensor the description of a sensor in an EntityType.
	Sensor struct {
		Range float32 `json:"range"`
		// TODO: Azimuth limits, active/passive, etc.
	}

	// Turret is the description of a turret's relative transform in an EntityType.
	Turret struct {
		PositionForward float32 `json:"positionForward"`
		PositionSide    float32 `json:"positionSide"`
		Angle           Angle   `json:"angle"`
		AzimuthFL       Angle   `json:"azimuthFL"`
		AzimuthFR       Angle   `json:"azimuthFR"`
		AzimuthBL       Angle   `json:"azimuthBL"`
		AzimuthBR       Angle   `json:"azimuthBR"`
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
	return armament.Type.Data().Reload
}

// Similar returns if the Armament is on the same turret and the same type as other.
func (armament *Armament) Similar(other *Armament) bool {
	return armament.Type == other.Type && armament.TurretIndex() == other.TurretIndex()
}

// Returns true only if the parameter is within the turrets valid azimuth ranges
func (turret *Turret) CheckAzimuth(curr Angle) bool {
	// Use floats so that negative angles work better with
	// comparison operators
	azimuthF := (Pi + curr - turret.Angle).Float()
	if turret.AzimuthFL.Float()-math32.Pi > azimuthF {
		return false
	}
	if math32.Pi-turret.AzimuthFR.Float() < azimuthF {
		return false
	}
	azimuthB := (curr - turret.Angle).Float()
	if turret.AzimuthBL.Float()-math32.Pi > azimuthB {
		return false
	}
	if math32.Pi-turret.AzimuthBR.Float() < azimuthB {
		return false
	}
	return true
}

func (sensors Sensors) MaxRange() float32 {
	return max(max(sensors.Visual.Range, sensors.Radar.Range), sensors.Sonar.Range)
}

func (sensors Sensors) Any() bool {
	return sensors.Visual.Range != 0 || sensors.Radar.Range != 0 || sensors.Sonar.Range != 0
}

func (entityType EntityType) Data() *EntityTypeData {
	return &entityTypeData[entityType]
}

// MaxHealth is the maximum health of an EntityType as Ticks.
func (entityType EntityType) MaxHealth() Ticks {
	data := entityType.Data()
	if data.Kind == EntityKindBoat {
		return DamageToTicks(entityType.Data().Damage)
	}
	// Arbitrary, small, non-zero value (TODO: maybe panic instead)
	return 20
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
	if LevelToScore(entityType.Data().Level+1) > score {
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

	return nextData.Level > data.Level && nextData.Kind == data.Kind && score >= LevelToScore(nextEntityType.Data().Level)
}

// levelToScore converts a boat level to a score required to upgrade
// score = (level^2 - 1) * 10
// js must have same function
func LevelToScore(level uint8) int {
	l := int(level)
	return (l*l - 1) * 10
}
