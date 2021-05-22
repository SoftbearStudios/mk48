// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

// extension is the extra fields required by boat entities
// EntityType in method signatures is required to not change without a call to setEntityType
// NOTE: This interface is NOT used at runtime; for efficiency, a concrete type is
type extension interface {
	setType(t EntityType)
	copiesAll() bool // Copies everything when anything is copied

	armamentConsumption(t EntityType) []Ticks // Read only
	copyArmamentConsumption(t EntityType)     // Copy for writes

	turretAngles(t EntityType) []Angle // Read only
	copyTurretAngles(t EntityType)     // Copy for writes

	turretTarget() Vec2f // Where turret want to point
	setTurretTarget(target Vec2f)

	altitude() float32
	setAltitude(float32)

	altitudeTarget() float32
	setAltitudeTarget(float32)

	damage() float32
	setDamage(float32)
}

func (entity *Entity) ArmamentConsumption() []Ticks {
	return entity.ext.armamentConsumption(entity.EntityType)
}

// -1 = deep, 0 = surface, 1 = high in the air
func (entity *Entity) Altitude() float32 {
	switch entity.EntityType.Data().Kind {
	case EntityKindBoat:
		return entity.ext.altitude()
	case EntityKindDecoy:
		switch entity.EntityType.Data().SubKind {
		case EntitySubKindSonar:
			return -0.9 * AltitudeCollisionThreshold
		}
	}

	switch entity.EntityType.Data().SubKind {
	case EntitySubKindTorpedo, EntitySubKindDepthCharge, EntitySubKindMine:
		// By multiplying by almost  negative one, these entities are allowed to
		// hit surface ships, but not much airborne things
		return -0.9 * AltitudeCollisionThreshold
	case EntitySubKindShell, EntitySubKindMissile, EntitySubKindRocket:
		// By multiplying by almost one, these entities are allowed to
		// hit surface ships, but not much underwater things
		return 0.9 * AltitudeCollisionThreshold
	default:
		return 0
	}
}

func (entity *Entity) SetAltitudeTarget(altitudeTarget float32) {
	entity.ext.setAltitudeTarget(clamp(altitudeTarget, -1, 1))
}

func (entity *Entity) TurretAngles() []Angle {
	return entity.ext.turretAngles(entity.EntityType)
}

func (entity *Entity) TurretTarget() Vec2f {
	return entity.ext.turretTarget()
}

func (entity *Entity) SetTurretTarget(target Vec2f) {
	entity.ext.setTurretTarget(target)
}
