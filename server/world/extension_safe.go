// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

type safeExtension struct {
	armaments       []Ticks // consumption of each armament
	angles          []Angle // angle of each turret
	target          Vec2f   // turret target position
	alt             float32 // altitude (see entity.Altitude for meaning)
	altTarget       float32 // desired altitude
	spawnProtection Ticks   // remaining
}

var _ = extension(&safeExtension{})

func (ext *safeExtension) setType(entityType EntityType) {
	data := entityType.Data()

	// Only keep target and target time
	*ext = safeExtension{target: ext.target, altTarget: ext.altTarget, spawnProtection: ext.spawnProtection}

	// Replenish all
	ext.armaments = make([]Ticks, len(data.Armaments))

	// Reset turrets to base positions
	turrets := data.Turrets
	ext.angles = make([]Angle, len(turrets))

	for i, turret := range turrets {
		ext.angles[i] = turret.Angle
	}
}

func (ext *safeExtension) copiesAll() bool {
	return false
}

func (ext *safeExtension) armamentConsumption() []Ticks {
	return ext.armaments
}

func (ext *safeExtension) copyArmamentConsumption() {
	a := make([]Ticks, len(ext.armaments))
	copy(a, ext.armaments)
	ext.armaments = a
}

func (ext *safeExtension) turretAngles() []Angle {
	return ext.angles
}

func (ext *safeExtension) copyTurretAngles() {
	ext.angles = copyAngles(ext.angles)
}

func (ext *safeExtension) turretTarget() Vec2f {
	return ext.target
}

func (ext *safeExtension) setTurretTarget(target Vec2f) {
	ext.target = target
}

func (ext *safeExtension) altitude() float32 {
	return ext.alt
}

func (ext *safeExtension) setAltitude(a float32) {
	ext.alt = a
}

func (ext *safeExtension) altitudeTarget() float32 {
	return ext.altTarget
}

func (ext *safeExtension) setAltitudeTarget(a float32) {
	ext.altTarget = a
}

func (ext *safeExtension) getSpawnProtection() Ticks {
	return ext.spawnProtection
}

func (ext *safeExtension) setSpawnProtection(val Ticks) {
	ext.spawnProtection = val
}
