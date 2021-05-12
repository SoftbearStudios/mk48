// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"github.com/chewxy/math32"
)

// Entity is an object in the world such as a boat, torpedo, crate or oil platform.
// Its size is 56 bytes + 8 bytes for entityID in sectorEntity = 64 bytes for optimal efficiency.
// Cannot modify EntityType directly.
type Entity struct {
	Transform
	Guidance
	EntityType
	Damage   float32
	Distance float32
	Lifespan float32 // In seconds
	Owner    *Player
	ext      unsafeExtension // Can be substituted for safeExtension with no other changes
}

// Update updates all the variables of an Entity such as Position, Direction, ArmamentConsumption etc.
// by an amount of time. It only modifies itself so each one can be processed by a different goroutine.
func (entity *Entity) Update(seconds float32, worldRadius float32, collider Collider) (die bool) {
	data := entity.Data()

	// Die
	if entity.Dead() {
		if owner := entity.Owner; owner != nil && entity.Data().Kind == EntityKindBoat {
			owner.DeathMessage = "You died!"
		}
		return true
	}

	entity.Lifespan += seconds
	if data.Lifespan != 0 && entity.Lifespan > data.Lifespan {
		return true
	}

	// Shells that have been added so far can't turn
	if data.SubKind != EntitySubKindShell && data.SubKind != EntitySubKindRocket {
		deltaAngle := entity.DirectionTarget.Diff(entity.Direction)
		entity.Direction += deltaAngle.ClampMagnitude(Angle(math32.Pi/4*max(0.25, 1-math32.Abs(entity.Velocity)/(data.Speed+1)))) * Angle(seconds)
	}

	if data.SubKind == EntitySubKindSubmarine {
		surfacing := false
		for _, consumption := range entity.ArmamentConsumption() {
			if consumption > 0.5 {
				surfacing = true
				break
			}
		}

		targetAltitude := clamp(entity.ext.altitudeTarget(), -1, 0)
		altitudeSpeed := float32(0.1)
		if surfacing {
			targetAltitude = 0
			altitudeSpeed = 1
		}
		entity.ext.setAltitude(Lerp(entity.Altitude(), targetAltitude, altitudeSpeed*seconds))
	}

	turretsCopied := entity.updateTurretAim(seconds)

	if entity.VelocityTarget > 0.01 || math32.Abs(entity.Velocity) > 0.01 {
		deltaVelocity := min(entity.VelocityTarget, data.Speed) - entity.Velocity
		entity.Velocity += seconds * deltaVelocity

		distance := seconds * entity.Velocity

		entity.Distance += distance
		if data.Range != 0 && entity.Distance > data.Range {
			return true
		}

		entity.Position = entity.Position.AddScaled(entity.Direction.Vec2f(), distance)

		// Test collisions with stationary objects

		if collider != nil && collider.Collides(entity, seconds) {
			if entity.Data().Kind != EntityKindBoat {
				return true
			}
			entity.Velocity = clampMagnitude(entity.Velocity, 5)
			if !(data.SubKind == EntitySubKindDredger || data.SubKind == EntitySubKindHovercraft) {
				entity.Damage += seconds * entity.MaxHealth() * 0.25
				if entity.Dead() {
					if owner := entity.Owner; owner != nil {
						owner.DeathMessage = "Crashed into the ground!"
					}
					return true
				}
			}
		}
	}

	// Radius can shrink
	if entity.Position.LengthSquared() > worldRadius*worldRadius {
		if owner := entity.Owner; owner != nil && entity.Data().Kind == EntityKindBoat {
			owner.DeathMessage = "Passed the border!"
		}
		return true
	}

	if len(entity.ArmamentConsumption()) > 0 {
		// If turrets were already copied and the extension
		// copies everything armaments don't need to be copied
		armamentsCopied := entity.ext.copiesAll() && turretsCopied
		entity.replenish(seconds*0.1, armamentsCopied)
	}

	if entity.Damage > 0 {
		entity.Repair(seconds * (1.0 / 60.0))
	}

	return false
}

// UpdateSensor runs a simple AI for homing torpedoes/missiles.
func (entity *Entity) UpdateSensor(otherEntity *Entity) {
	if entity.Owner.Friendly(otherEntity.Owner) {
		return
	}
	if entity.Distance < 25 || entity.Lifespan < 0.5 {
		// Sensor not active yet
		return
	}
	diff := otherEntity.Position.Sub(entity.Position)
	angle := diff.Angle()

	angleTargetDiff := entity.DirectionTarget.Diff(angle).Abs()
	if angleTargetDiff > Angle(math32.Pi/3) {
		// Should not go off target
		return
	}

	angleDiff := entity.Direction.Diff(angle).Abs()
	if angleDiff > Angle(math32.Pi/3) {
		// Cannot sense beyond this angle
		return
	}

	homingStrength := otherEntity.Data().Radius * 600 / (1 + diff.LengthSquared() + 20000*square(float32(angleDiff)))

	entity.DirectionTarget = entity.DirectionTarget.Lerp(angle, min(0.95, max(0.01, homingStrength)))
}

// Returns whether copied turret angles
func (entity *Entity) updateTurretAim(seconds float32) bool {
	turretsCopied := false

	// Don't rotate turret if aim first is semi-fresh
	turretTargetTime := entity.TurretTargetTime()
	data := entity.Data()
	if entity.Lifespan < turretTargetTime+1 || entity.Lifespan > turretTargetTime+5 {
		for i := range entity.TurretAngles() {
			turretData := data.Turrets[i]
			directionTarget := turretData.Angle
			if entity.Lifespan < turretTargetTime+5 { // turret target lasts for 5 seconds
				turretGlobalTransform := entity.Transform.Add(Transform{
					Position: Vec2f{
						X: turretData.PositionForward,
						Y: turretData.PositionSide,
					},
					Direction: entity.TurretAngles()[i],
				})
				globalDirection := entity.TurretTarget().Sub(turretGlobalTransform.Position).Angle()
				directionTarget = globalDirection - entity.Direction
			}
			deltaAngle := directionTarget.Diff(entity.TurretAngles()[i])
			angle := deltaAngle.ClampMagnitude(Angle(seconds))

			if angle != 0 {
				// Copy on write
				if !turretsCopied {
					turretsCopied = true
					entity.ext.copyTurretAngles(entity.EntityType)
				}
				entity.TurretAngles()[i] += angle
			}
		}
	}

	return turretsCopied
}

// Repair regenerates the Entity's health by an amount.
func (entity *Entity) Repair(amount float32) {
	entity.Damage -= min(amount, entity.Damage)
}

// Replenish replenishes the Entity's armaments by an amount.
// It starts with the ones that have the least time left.
// O(n^2) worst case if amount is very high.
func (entity *Entity) Replenish(amount float32) {
	if len(entity.ArmamentConsumption()) == 0 {
		return // don't crash
	}
	entity.replenish(amount, false)
}

// replenish is a helper to that can avoid copying turret angles and armaments for unsafeExtension.
// It replenishes each range of Similar Armaments.
func (entity *Entity) replenish(amount float32, copied bool) {
	armaments := entity.Data().Armaments
	current := &armaments[0]
	start := 0

	for end := range armaments {
		if next := &armaments[end]; !next.Similar(current) {
			current = next
			copied = entity.replenishRange(amount, start, end, copied)
			start = end
		}
	}

	// Final iteration
	copied = entity.replenishRange(amount, start, len(armaments), copied)
}

// replenishRange replenishes a range of armaments and returns copied.
func (entity *Entity) replenishRange(amount float32, start, end int, copied bool) bool {
	for amount > 0 {
		i := -1
		consumption := float32(math32.MaxFloat32)
		for j, c := range entity.ArmamentConsumption()[start:end] {
			if c > 0.0 && c < consumption {
				i = j + start
				consumption = c
			}
		}

		// All replenished
		if i == -1 {
			break
		} else if !copied {
			copied = true
			entity.ext.copyArmamentConsumption(entity.EntityType)
		}

		entity.ArmamentConsumption()[i] = max(0, consumption-amount)
		amount -= consumption
	}

	return copied
}

// Dead returns if an Entity's health is less than 0.
func (entity *Entity) Dead() bool {
	return entity.Damage > entity.MaxHealth()
}

// Health returns an Entity's health as an absolute.
func (entity *Entity) Health() float32 {
	return entity.MaxHealth() - entity.Damage
}

// DamagePercent returns an Entity's damage in the range [0, 1.0].
func (entity *Entity) DamagePercent() float32 {
	return entity.Damage / entity.MaxHealth()
}

// HealthPercent returns an Entity's health in the range [0, 1.0].
func (entity *Entity) HealthPercent() float32 {
	return 1 - entity.DamagePercent()
}

func (entity *Entity) HasArmament(index int) bool {
	return HasArmament(entity.ArmamentConsumption(), index)
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
	if weaponData.SubKind == EntitySubKindShell {
		// Model shells with instantaneous accelerations to muzzle velocity
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
	if entity.Data().Kind == EntityKindBoat && entity.Owner != nil {
		if entity.Owner.EntityID == EntityIDInvalid {
			panic("not player's entity")
		}
		entity.Owner.Died(entity)
		entity.Owner.EntityID = EntityIDInvalid
	}
}

func HasArmament(consumption []float32, index int) bool {
	return len(consumption) <= index || consumption[index] < 0.000001
}

func (entity *Entity) ConsumeArmament(index int) {
	entity.ext.copyArmamentConsumption(entity.EntityType)
	consumption := float32(1)
	switch entity.Data().Armaments[index].Default.Data().SubKind {
	case EntitySubKindDredger:
		consumption = 0.1
	case EntitySubKindRocket:
		consumption = 0.25
	}
	entity.ArmamentConsumption()[index] = consumption
}

// Initialize is called whenever a boat's type is set/changed.
func (entity *Entity) Initialize(entityType EntityType) {
	entity.EntityType = entityType
	entity.ext.setType(entity.EntityType)

	// Make sure all the new turrets are re-aimed to the old target.
	entity.updateTurretAim(5)

	// Starting depth
	switch entityType.Data().SubKind {
	case EntitySubKindSubmarine:
		entity.ext.setAltitude(-0.25)
	default:
		entity.ext.setAltitude(0)
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
