// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

// extension is the extra fields required by boat entities
// EntityType in method signatures is required to not change without a call to setEntityType
// NOTE: This interface is NOT used at runtime; for efficiency, a concrete type is
type extension interface {
	setType(t EntityType)
	copiesAll() bool // Copies everything when anything is copied

	armamentConsumption(t EntityType) []float32 // Read only
	copyArmamentConsumption(t EntityType)       // Copy for writes

	turretAngles(t EntityType) []Angle // Read only
	copyTurretAngles(t EntityType)     // Copy for writes

	turretTarget() Vec2f // Where turret want to point
	setTurretTarget(target Vec2f)

	turretTargetTime() float32 // Time in terms of lifespan when turret target was set
	setTurretTargetTime(t float32)

	altitude() float32
	setAltitude(float32)
}

func (entity *Entity) ArmamentConsumption() []float32 {
	return entity.ext.armamentConsumption(entity.EntityType)
}

// Entities within this of eachother's altitudes can collide
const AltitudeCollisionThreshold = 0.25

// -1 = deep, 0 = surface, 1 = high in the air
func (entity *Entity) Altitude() float32 {
	if entity.EntityType.Data().Kind == EntityKindBoat {
		return entity.ext.altitude()
	}

	switch entity.EntityType.Data().SubKind {
	case EntitySubKindTorpedo, EntitySubKindDepthCharge:
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

func (entity *Entity) TurretAngles() []Angle {
	return entity.ext.turretAngles(entity.EntityType)
}

func (entity *Entity) TurretTarget() Vec2f {
	return entity.ext.turretTarget()
}

func (entity *Entity) SetTurretTarget(target Vec2f) {
	entity.ext.setTurretTarget(target)
}

func (entity *Entity) TurretTargetTime() float32 {
	return entity.ext.turretTargetTime()
}

func (entity *Entity) SetTurretTargetTime(t float32) {
	entity.ext.setTurretTargetTime(t)
}
