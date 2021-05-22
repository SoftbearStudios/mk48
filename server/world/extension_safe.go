// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

type safeExtension struct {
	armaments []Ticks // consumption of each armament
	angles    []Angle // angle of each turret
	target    Vec2f   // turret target position
	alt       float32 // altitude (see entity.Altitude for meaning)
	altTarget float32 // desired altitude
	time      Ticks   // entity lifespan when last aimed turrets
}

var _ = extension(&safeExtension{})

func (ext *safeExtension) setType(entityType EntityType) {
	data := entityType.Data()

	// Only keep target and target time
	*ext = safeExtension{target: ext.target, time: ext.time, altTarget: ext.altTarget}

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

func (ext *safeExtension) armamentConsumption(_ EntityType) []Ticks {
	return ext.armaments
}

func (ext *safeExtension) copyArmamentConsumption(_ EntityType) {
	a := make([]Ticks, len(ext.armaments))
	copy(a, ext.armaments)
	ext.armaments = a
}

func (ext *safeExtension) turretAngles(_ EntityType) []Angle {
	return ext.angles
}

func (ext *safeExtension) copyTurretAngles(_ EntityType) {
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
