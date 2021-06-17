// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import "github.com/chewxy/math32"

// extension is the extra fields required by boat entities
// EntityType in method signatures is required to not change without a call to setEntityType
// NOTE: This interface is NOT used at runtime; for efficiency, a concrete type is
type extension interface {
	setType(t EntityType)
	copiesAll() bool // Copies everything when anything is copied

	armamentConsumption() []Ticks // Read only
	copyArmamentConsumption()     // Copy for writes

	turretAngles() []Angle // Read only
	copyTurretAngles()     // Copy for writes

	aimTarget() Vec2f // Where turret wants to point
	setAimTarget(target Vec2f)

	altitude() float32
	setAltitude(float32)

	altitudeTarget() float32
	setAltitudeTarget(float32)
}

func (entity *Entity) ArmamentConsumption() []Ticks {
	entity.mustBoat()
	return entity.Owner.ext.armamentConsumption()
}

// -1 = deep, 0 = surface, 1 = high in the air
func (entity *Entity) Altitude() float32 {
	data := entity.Data()
	switch data.Kind {
	case EntityKindAircraft:
		return 1.8 * AltitudeCollisionThreshold
	case EntityKindBoat:
		return entity.Owner.ext.altitude()
	case EntityKindDecoy:
		switch entity.EntityType.Data().SubKind {
		case EntitySubKindSonar:
			return -0.9 * AltitudeCollisionThreshold
		}
	}

	switch data.SubKind {
	case EntitySubKindTorpedo, EntitySubKindDepthCharge, EntitySubKindMine:
		// By multiplying by almost  negative one, these entities are allowed to
		// hit surface ships, but not much airborne things
		return -0.9 * AltitudeCollisionThreshold
	case EntitySubKindShell, EntitySubKindMissile, EntitySubKindRocket:
		if len(data.Armaments) == 0 {
			// By multiplying by almost one, these entities are allowed to
			// hit surface ships, but not much underwater things
			return 0.9 * AltitudeCollisionThreshold
		}
		fallthrough // ASROC
	case EntitySubKindSAM:
		return 1.8 * AltitudeCollisionThreshold
	default:
		return 0
	}
}

// Returns value in the range [0,1] where 0 is full protection and 1 is no protection
// Intended to be multiplied by, for example, damage, to decrease damage taken
// while having been spawned recently
func (entity *Entity) SpawnProtection() float32 {
	entity.mustBoat()
	remaining := entity.Owner.ext.getSpawnProtection()
	ret := (spawnProtection - remaining).Float() / spawnProtection.Float()
	if ret > 1 {
		panic("spawn protection would be too high")
	}
	return ret
}

func (entity *Entity) ClearSpawnProtection() {
	entity.mustBoat()
	entity.Owner.ext.setSpawnProtection(0)
}

func (entity *Entity) SetAltitudeTarget(altitudeTarget float32) {
	entity.mustBoat()
	entity.Owner.ext.setAltitudeTarget(clamp(altitudeTarget, -1, 1))
}

func (entity *Entity) TurretAngles() []Angle {
	entity.mustBoat()
	return entity.Owner.ext.turretAngles()
}

func (entity *Entity) AimTarget() Vec2f {
	entity.mustBoat()
	return entity.Owner.ext.aimTarget()
}

// When a weapon wants the turret target of the owner's ship
func (entity *Entity) OwnerBoatAimTarget() Vec2f {
	if entity.Owner == nil {
		return Vec2f{}
	}
	return entity.Owner.ext.aimTarget()
}

func (entity *Entity) SetAimTarget(target Vec2f) {
	entity.mustBoat()

	// Clamp to within visual radius.
	_, visual, _, _ := entity.Camera()
	r2 := visual * visual

	diff := target.Sub(entity.Position)
	if d2 := diff.LengthSquared(); d2 > r2 {
		distance := math32.Sqrt(d2)
		normal := diff.Div(distance)
		target = target.AddScaled(normal, visual-distance)
	}

	entity.Owner.ext.setAimTarget(target)
}

// Call when accessing entity.Owner.ext, which is ONLY valid
// on the owner's boat entity
func (entity *Entity) mustBoat() {
	if entity.Data().Kind != EntityKindBoat {
		panic("access extension of non-boat")
	}
}
