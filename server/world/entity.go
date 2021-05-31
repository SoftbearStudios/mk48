// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"github.com/chewxy/math32"
)

// Entity is an object in the world such as a boat, torpedo, crate or oil platform.
// Its size is 32 bytes for optimal efficiency.
// Cannot modify EntityType directly.
type Entity struct {
	Transform
	Guidance
	Owner *Player
	EntityType
	Lifespan Ticks
	EntityID EntityID
}

// Update updates all the variables of an Entity such as Position, Direction, ArmamentConsumption etc.
// by an amount of time. It only modifies itself so each one can be processed by a different goroutine.
// seconds cannot be > 1.0.
func (entity *Entity) Update(ticks Ticks, worldRadius float32, collider Collider) (die bool) {
	data := entity.Data()

	entity.Lifespan += ticks
	if data.Lifespan != 0 && entity.Lifespan > data.Lifespan {
		if entity.EntityType == EntityTypeHQ {
			// Downgrade
			entity.EntityType = EntityTypeOilPlatform
		} else {
			// Die
			return true
		}
	}

	// The following movement-related code must match the client's code
	maxSpeed := data.Speed
	seconds := ticks.Float()

	if data.Limited && entity.Owner.DeathTime != 0 {
		return true
	}

	if data.SubKind == EntitySubKindAircraft {
		posTarget := entity.OwnerBoatTurretTarget()
		posDiff := posTarget.Sub(entity.Position)

		// Vary angle based on entity hash so aircraft doesn't clump as much.
		entity.DirectionTarget = posDiff.Angle() + ToAngle(entity.Hash()*math32.Pi/4) - Pi/8

		// Let other aircraft catch up
		if posDiff.LengthSquared() < 75*75 || entity.Direction.Diff(entity.DirectionTarget).Abs() > math32.Pi/3 {
			maxSpeed -= 30 * MeterPerSecond
		}
	}

	// Shells that have been added so far can't turn
	if data.SubKind != EntitySubKindShell && data.SubKind != EntitySubKindRocket {
		deltaAngle := entity.DirectionTarget.Diff(entity.Direction)

		// See #45 - automatically slow down to turn faster
		maxSpeedF := maxSpeed.Float()
		turnRate := math32.Pi / 4

		if data.SubKind != EntitySubKindAircraft {
			maxSpeedF /= max(square(deltaAngle.Abs()), 1)
			turnRate *= max(0.25, 1-math32.Abs(entity.Velocity.Float())/(maxSpeed.Float()+1))
		}
		maxSpeed = ToVelocity(maxSpeedF)
		entity.Direction += deltaAngle.ClampMagnitude(ToAngle(seconds * turnRate))
	}

	if data.SubKind == EntitySubKindSubmarine {
		ext := &entity.Owner.ext
		targetAltitude := clamp(ext.altitudeTarget(), -1, 0)
		altitudeSpeed := float32(0.25)
		altitudeChange := clampMagnitude(targetAltitude-entity.Altitude(), altitudeSpeed*seconds)
		ext.setAltitude(entity.Altitude() + altitudeChange)
	}

	var turretsCopied bool
	if data.Kind == EntityKindBoat {
		turretsCopied = entity.updateTurretAim(ToAngle(seconds))
	}

	if entity.VelocityTarget != 0 || entity.Velocity != 0 {
		deltaVelocity := entity.VelocityTarget.ClampMagnitude(maxSpeed) - entity.Velocity
		deltaVelocity = deltaVelocity.ClampMagnitude(ToVelocity(800 * seconds)) // max acceleration
		entity.Velocity += ToVelocity(seconds * deltaVelocity.Float())
		entity.Position = entity.Position.AddScaled(entity.Direction.Vec2f(), seconds*entity.Velocity.Float())

		// Test collisions with stationary objects
		if collider != nil && collider.Collides(entity, seconds) {
			if entity.Data().Kind != EntityKindBoat {
				return true
			}
			entity.Velocity = entity.Velocity.ClampMagnitude(5 * MeterPerSecond)
			if !(data.SubKind == EntitySubKindDredger || data.SubKind == EntitySubKindHovercraft) {
				if entity.Damage(seconds * entity.MaxHealth() * 0.25) {
					if owner := entity.Owner; owner != nil {
						owner.DeathMessage = "Crashed into the ground!"
					}
					return true
				}
			}
		}
	}

	// Border (note: radius can shrink)
	centerDist2 := entity.Position.LengthSquared()
	if centerDist2 > square(worldRadius) {
		dead := entity.Damage(seconds * entity.MaxHealth())
		entity.Velocity += ToVelocity(clampMagnitude(entity.Velocity.Float()-6*entity.Position.Dot(entity.Direction.Vec2f()), 15))
		// Everything but boats is instantly killed by border
		if dead || data.Kind != EntityKindBoat || centerDist2 > square(worldRadius*RadiusClearance) {
			if owner := entity.Owner; owner != nil && entity.Data().Kind == EntityKindBoat {
				owner.DeathMessage = "Crashed into the border!"
			}
			return true
		}
	}

	if data.Kind == EntityKindBoat {
		if len(entity.ArmamentConsumption()) > 0 {
			// If turrets were already copied and the extension
			// copies everything armaments don't need to be copied
			armamentsCopied := entity.Owner.ext.copiesAll() && turretsCopied
			entity.replenish(ticks, armamentsCopied)
		}

		if ext := &entity.Owner.ext; ext.damage() > 0 {
			repairAmount := seconds * (1.0 / 60.0)
			if ext.alt < 0 {
				repairAmount *= 0.5
			}
			entity.Repair(repairAmount)
		}
	}

	return false
}

// Damage damages an entity and returns if it died.
func (entity *Entity) Damage(d float32) bool {
	data := entity.Data()
	if data.Kind != EntityKindBoat {
		return d > 0.0
	}

	ext := &entity.Owner.ext
	d += ext.damage()
	ext.setDamage(d)
	return d > entity.MaxHealth()
}

// UpdateSensor runs a simple AI for homing torpedoes/missiles.
func (entity *Entity) UpdateSensor(otherEntity *Entity) {
	if entity.Owner.Friendly(otherEntity.Owner) {
		return
	}
	if entity.Lifespan < 1 {
		// Sensor not active yet
		return
	}

	data := entity.Data()
	otherData := otherEntity.Data()

	var relevant bool
	var baseHomingStrength float32 = 600

	switch data.SubKind {
	case EntitySubKindSAM:
		baseHomingStrength = 10000
		relevant = otherData.SubKind == EntitySubKindAircraft || otherData.SubKind == EntitySubKindMissile || otherData.SubKind == EntitySubKindRocket
	default:
		relevant = otherData.Kind == EntityKindBoat || otherData.Kind == EntityKindDecoy
	}

	if !relevant {
		return
	}

	diff := otherEntity.Position.Sub(entity.Position)
	angle := diff.Angle()

	angleTargetDiff := entity.DirectionTarget.Diff(angle).Abs()
	if angleTargetDiff > math32.Pi/3 {
		// Should not go off target
		return
	}

	angleDiff := entity.Direction.Diff(angle).Abs()
	if angleDiff > math32.Pi/3 {
		// Cannot sense beyond this angle
		return
	}

	size := otherData.Radius
	if otherData.Kind == EntityKindDecoy {
		// Decoys appear very large to weapons
		size = 100
	}

	homingStrength := size * baseHomingStrength / (1 + diff.LengthSquared() + 20000*square(angleDiff))
	entity.DirectionTarget = entity.DirectionTarget.Lerp(angle, min(0.95, max(0.01, homingStrength)))
}

// Returns a float in range [0, 1) based on the entity's id.
func (entity *Entity) Hash() float32 {
	const hashSize = 64
	return float32(entity.EntityID&(hashSize-1)) * (1.0 / hashSize)
}

// Returns value in the range [0,1] as time since spawn increases
// Intended to be multiplied by, for example, damage, to decrease damage taken
// while having been spawned recently
func (entity *Entity) RecentSpawnFactor() float32 {
	// Upgrading invalidates spawn protection
	if entity.Data().Level > 1 {
		return 1
	}
	const initial float32 = 0.75 // initial protection against damage, etc.
	const seconds float32 = 15   // how long effect lasts
	return min(1-initial+entity.Lifespan.Float()*(initial/seconds), 1)
}

// Returns whether copied turret angles
func (entity *Entity) updateTurretAim(amount Angle) bool {
	turretsCopied := false
	data := entity.Data()
	angles := entity.TurretAngles()

	for i := range angles {
		turretData := data.Turrets[i]
		directionTarget := turretData.Angle
		if target := entity.TurretTarget(); target != (Vec2f{}) { // turret target lasts for 5 seconds
			turretGlobalTransform := entity.Transform.Add(Transform{
				Position: Vec2f{
					X: turretData.PositionForward,
					Y: turretData.PositionSide,
				},
				Direction: angles[i],
			})
			globalDirection := target.Sub(turretGlobalTransform.Position).Angle()
			directionTarget = globalDirection - entity.Direction
		}
		deltaAngle := directionTarget.Diff(angles[i])
		if amount < Pi {
			deltaAngle = deltaAngle.ClampMagnitude(amount)
		}
		if deltaAngle != 0 {
			// Copy on write
			if !turretsCopied {
				turretsCopied = true
				entity.Owner.ext.copyTurretAngles()
			}
			entity.TurretAngles()[i] += deltaAngle
		}
	}

	return turretsCopied
}

// Repair regenerates the Entity's health by an amount.
func (entity *Entity) Repair(amount float32) {
	entity.mustBoat()
	ext := &entity.Owner.ext
	damage := ext.damage()
	if amount >= damage {
		ext.setDamage(0)
	} else {
		ext.setDamage(damage - amount)
	}
}

// Replenish replenishes the Entity's armaments by an amount.
// It starts with the ones that have the least time left.
// O(n^2) worst case if amount is very high.
func (entity *Entity) Replenish(amount Ticks) {
	if len(entity.ArmamentConsumption()) == 0 {
		return // don't crash
	}
	entity.replenish(amount, false)
}

// replenish is a helper to that can avoid copying turret angles and armaments for unsafeExtension.
// It replenishes each range of Similar Armaments.
func (entity *Entity) replenish(amount Ticks, copied bool) {
	armaments := entity.Data().Armaments
	current := &armaments[0]
	start := 0

	for end := range armaments {
		if next := &armaments[end]; !next.Similar(current) {
			copied = entity.replenishRange(amount, start, end, copied)
			current = next
			start = end
		}
	}

	// Final iteration
	copied = entity.replenishRange(amount, start, len(armaments), copied)
}

// replenishRange replenishes a range of armaments and returns copied.
func (entity *Entity) replenishRange(amount Ticks, start, end int, copied bool) bool {
	for amount != 0 {
		i := -1
		// Limited are ticks max and won't be counted
		consumption := TicksMax
		for j, c := range entity.ArmamentConsumption()[start:end] {
			if c != 0 && c < consumption {
				i = j + start
				consumption = c
			}
		}

		// All replenished
		if i == -1 {
			break
		} else if !copied {
			copied = true
			entity.Owner.ext.copyArmamentConsumption()
		}

		if consumption < amount {
			consumption = 0
			amount -= consumption
		} else {
			consumption -= amount
			amount = 0
		}

		entity.ArmamentConsumption()[i] = consumption
	}

	return copied
}

// DamagePercent returns an Entity's damage in the range [0, 1.0].
func (entity *Entity) DamagePercent() float32 {
	if entity.Data().Kind == EntityKindBoat {
		return entity.Owner.ext.damage() / entity.MaxHealth()
	}
	return 0
}

// HealthPercent returns an Entity's health in the range [0, 1.0].
func (entity *Entity) HealthPercent() float32 {
	return 1 - entity.DamagePercent()
}

// ArmamentTransform returns the world transform of an Armament.
func ArmamentTransform(entityType EntityType, entityTransform Transform, turretAngles []Angle, index int) Transform {
	armamentData := entityType.Data().Armaments[index]
	transform := Transform{
		Position: Vec2f{
			X: armamentData.PositionForward,
			Y: armamentData.PositionSide,
		},
		Direction: armamentData.Angle,
	}

	weaponData := armamentData.Default.Data()

	// Shells start with all their velocity.
	if weaponData.SubKind == EntitySubKindShell {
		transform.Velocity = weaponData.Speed
	}

	if armamentData.Turret != nil {
		turretData := entityType.Data().Turrets[*armamentData.Turret]
		transform = Transform{
			Position: Vec2f{
				X: turretData.PositionForward,
				Y: turretData.PositionSide,
			},
			Direction: turretAngles[*armamentData.Turret],
		}.Add(transform)
	}
	return entityTransform.Add(transform)
}

// ArmamentTransform Returns world transform
func (entity *Entity) ArmamentTransform(index int) Transform {
	return ArmamentTransform(entity.EntityType, entity.Transform, entity.TurretAngles(), index)
}

// Close is called when an Entity is removed from a World.
func (entity *Entity) Close() {
	data := entity.Data()
	if data.Kind == EntityKindBoat && entity.Owner != nil {
		if entity.Owner.EntityID == EntityIDInvalid {
			panic("not player's entity")
		}
		entity.Owner.Died(entity)
		entity.Owner.EntityID = EntityIDInvalid
		entity.Owner.ext = unsafeExtension{}
	} else if data.Kind == EntityKindWeapon {
		// Regen limited armament
		if data.Limited && entity.Owner != nil {
			consumption := entity.Owner.ext.armamentConsumption()
			armaments := entity.Owner.ext.typ.Data().Armaments

			// Reload 1 limited armament of its type.
			for i := range armaments {
				a := &armaments[i]
				if a.Default == entity.EntityType && consumption[i] == TicksMax {
					consumption[i] = a.Reload()
					return
				}
			}
		}
	}
}

func (entity *Entity) ConsumeArmament(index int) {
	entity.Owner.ext.copyArmamentConsumption()
	a := &entity.Data().Armaments[index]

	// Limited armaments start their timer when they die.
	reload := TicksMax
	if !a.Default.Data().Limited {
		reload = a.Reload()

		// Submerged submarines reload slower
		if entity.Owner.ext.altitude() < 0 {
			reload *= 2
		}
	}

	entity.ArmamentConsumption()[index] = reload
}

// Initialize is called whenever a boat's type is set/changed.
func (entity *Entity) Initialize(entityType EntityType) {
	var oldArmaments []Ticks
	var oldDamage float32
	if entity.EntityType != EntityTypeInvalid {
		oldArmaments = entity.ArmamentConsumption()
		oldDamage = entity.DamagePercent()
	}

	ext := &entity.Owner.ext

	entity.EntityType = entityType
	ext.setType(entity.EntityType)

	// Keep similar consumption.
	upgradeArmaments(entityType, entity.ArmamentConsumption(), oldArmaments)
	ext.setDamage(oldDamage * 0.5 * entity.MaxHealth())

	// Make sure all the new turrets are re-aimed to the old target.
	entity.updateTurretAim(Pi)

	// Starting depth
	switch entityType.Data().SubKind {
	case EntitySubKindSubmarine:
		ext.setAltitude(-0.5)
	default:
		ext.setAltitude(0)
	}
}

// upgradeArmaments attempts to keep a similar amount of consumption when upgrading.
// See #32 for rationale.
func upgradeArmaments(entityType EntityType, new []Ticks, old []Ticks) {
	if len(new) == 0 || len(old) == 0 {
		return
	}

	// Use uint to prevent overflow.
	var sum uint
	for _, c := range old {
		// Not limited
		if c != TicksMax {
			sum += uint(c)
		}
	}

	if sum == 0 {
		return
	}

	// Use same armament consumption ticks.
	armaments := entityType.Data().Armaments
	for i := range new {
		reload := armaments[i].Reload()
		if sum > uint(reload) {
			new[i] = reload
			sum -= uint(reload)
		} else {
			new[i] = Ticks(sum)
			break
		}
	}
}

// Camera is the combined Sensor view of an Entity.
func (entity *Entity) Camera() (position Vec2f, visual, radar, sonar float32) {
	for _, sensor := range entity.Data().Sensors {
		switch sensor.Type {
		case SensorTypeRadar:
			radar = max(radar, sensor.Range)
		case SensorTypeSonar:
			sonar = max(sonar, sensor.Range)
		case SensorTypeVisual:
			visual = max(visual, sensor.Range)
		}
	}
	position = entity.Position

	// High altitude benefits radar and visual, low altitude diminishes them
	alt := entity.Altitude()
	visual *= clamp(alt+1, 0.5, 1) // hack to allow basic vision
	radar *= min(alt, 0) + 1

	// Sonar doesn't work at all out of water
	if alt > 0 {
		sonar = 0
	}
	return
}
