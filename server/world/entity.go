// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"github.com/chewxy/math32"
)

const spawnProtection Ticks = 10 * TicksPerSecond

// Entity is an object in the world such as a boat, torpedo, crate or oil platform.
// Its size is 32 bytes for optimal efficiency.
// Cannot modify EntityType directly.
// Entity.Ticks is either damage or lifespan depending on the entity's type.
// Cannot modify EntityID except in World.
type Entity struct {
	Transform
	Guidance
	Owner *Player
	EntityType
	Ticks    Ticks
	EntityID EntityID
}

// Update updates all the variables of an Entity such as Position, Direction, ArmamentConsumption etc.
// by an amount of time. It only modifies itself so each one can be processed by a different goroutine.
// seconds cannot be > 1.0.
func (entity *Entity) Update(ticks Ticks, worldRadius float32, collider Collider) (die bool) {
	data := entity.Data()

	if lifespan := data.Lifespan; lifespan != 0 {
		entity.Ticks += ticks

		// Downgrade or die when expired.
		if entity.Ticks > lifespan {
			if entity.EntityType == EntityTypeHQ {
				entity.EntityType = EntityTypeOilPlatform
			} else {
				return true
			}
		}
	}

	// Limited entities die when their owner does.
	if data.Limited && entity.Owner.DeathTime != 0 {
		return true
	}

	// The following movement-related code must match the client's code
	maxSpeed := data.Speed
	seconds := ticks.Float()

	if data.SubKind == EntitySubKindAircraft {
		posTarget := entity.OwnerBoatAimTarget()
		posDiff := posTarget.Sub(entity.Position)

		// Vary angle based on entity hash so aircraft doesn't clump as much.
		entity.DirectionTarget = posDiff.Angle() + ToAngle(entity.Hash()*math32.Pi/4) - Pi/8
		distance := posDiff.LengthSquared()

		// Probably will have heli sub-kind in future.
		if entity.EntityType == EntityTypeSeahawk {
			if distance < 35*35 {
				maxSpeed = 0
			}
		} else {
			// Let other aircraft catch up
			if distance < 75*75 || entity.Direction.Diff(entity.DirectionTarget).Abs() > math32.Pi/3 {
				maxSpeed -= 30 * MeterPerSecond
			}
		}
	} else if data.SubKind == EntitySubKindSubmarine {
		ext := &entity.Owner.ext
		targetAltitude := clamp(ext.altitudeTarget(), -1, 0)
		altitudeSpeed := float32(0.25)
		altitudeChange := clampMagnitude(targetAltitude-entity.Altitude(), altitudeSpeed*seconds)
		ext.setAltitude(entity.Altitude() + altitudeChange)
	}

	boat := data.Kind == EntityKindBoat
	if boat {
		// Spawn protection
		sp := entity.Owner.ext.getSpawnProtection()
		toSubtract := ticks
		// Avoid overflow of unsigned ticks
		if toSubtract > sp {
			toSubtract = sp
		}
		entity.Owner.ext.setSpawnProtection(sp - toSubtract)

		turretsCopied := entity.updateTurretAim(ToAngle(seconds * (math32.Pi / 3)))
		if len(entity.ArmamentConsumption()) > 0 {
			// If turrets were already copied and the extension
			// copies everything armaments don't need to be copied
			armamentsCopied := entity.Owner.ext.copiesAll() && turretsCopied
			entity.replenish(ticks, armamentsCopied)
		}

		entity.Repair(ticks)
	}

	// Shells that have been added so far can't turn
	if data.SubKind != EntitySubKindShell && data.SubKind != EntitySubKindRocket {
		// See #45 - automatically slow down to turn faster
		turnRate := math32.Pi / 4
		if entity.EntityType == EntityTypeSeahawk {
			turnRate = math32.Pi / 2
		}
		deltaAngle := entity.DirectionTarget.Diff(entity.Direction)

		if data.SubKind != EntitySubKindAircraft {
			maxSpeed = ToVelocity(maxSpeed.Float() / max(square(deltaAngle.Float()), 1))
			turnRate *= max(0.25, 1-math32.Abs(entity.Velocity.Float())/(maxSpeed.Float()+1))
		}

		deltaAngle = deltaAngle.ClampMagnitude(ToAngle(seconds * turnRate))
		entity.Direction += deltaAngle
	}

	if entity.VelocityTarget != 0 || entity.Velocity != 0 || boat {
		deltaVelocity := entity.VelocityTarget.ClampMagnitude(maxSpeed) - entity.Velocity
		if deltaVelocity != 0 {
			deltaVelocity = deltaVelocity.ClampMagnitude(ToVelocity(800 * seconds)) // Max acceleration
			entity.Velocity += ToVelocity(seconds * deltaVelocity.Float()).ClampMin(1)
		}
		entity.Position = entity.Position.AddScaled(entity.Direction.Vec2f(), seconds*entity.Velocity.Float())

		// Test collisions with stationary objects
		if collider != nil && collider.Collides(entity, seconds) {
			if entity.Data().Kind != EntityKindBoat {
				return true
			}
			entity.Velocity = entity.Velocity.ClampMagnitude(5 * MeterPerSecond)
			if !(data.SubKind == EntitySubKindDredger || data.SubKind == EntitySubKindHovercraft) {
				if entity.KillIn(ticks, 4*TicksPerSecond) {
					if owner := entity.Owner; owner != nil {
						owner.DeathReason = DeathReason{Type: DeathTypeTerrain}
					}
					return true
				}
			}
		}
	}

	// Border (note: radius can shrink)
	centerDist2 := entity.Position.LengthSquared()
	if centerDist2 > square(worldRadius) {
		dead := entity.KillIn(ticks, 1*TicksPerSecond)
		entity.Velocity += ToVelocity(clampMagnitude(entity.Velocity.Float()-6*entity.Position.Dot(entity.Direction.Vec2f()), 15))
		// Everything but boats is instantly killed by border
		if dead || data.Kind != EntityKindBoat || centerDist2 > square(worldRadius*RadiusClearance) {
			if owner := entity.Owner; owner != nil && entity.Data().Kind == EntityKindBoat {
				owner.DeathReason = DeathReason{Type: DeathTypeBorder}
			}
			return true
		}
	}

	return false
}

// Damage damages an entity and returns if it died.
func (entity *Entity) Damage(damage Ticks) bool {
	// Ticks is lifespan for non-boats.
	if entity.Data().Kind != EntityKindBoat {
		return damage != 0
	}

	// Don't overflow
	if int(entity.Ticks)+int(damage) > int(entity.MaxHealth()) {
		return true
	}

	entity.Ticks += damage
	return false
}

// KillIn damages an entity enough to kill it in killTime Ticks.
func (entity *Entity) KillIn(ticks, killTime Ticks) bool {
	return entity.Damage(ticks * (entity.MaxHealth() / killTime).ClampMin(1))
}

// UpdateSensor runs a simple AI for homing torpedoes/missiles.
func (entity *Entity) UpdateSensor(otherEntity *Entity) {
	if entity.Owner.Friendly(otherEntity.Owner) {
		return
	}

	// Sensor activates after 1 second and when the direction target is reached
	if entity.Ticks < TicksPerSecond && entity.Direction.Diff(entity.DirectionTarget) > ToAngle(0.15) {
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
	if angleTargetDiff > math32.Pi/6 {
		// Should not go off target
		return
	}

	angleDiff := entity.Direction.Diff(angle).Abs()
	if angleDiff > math32.Pi/5 {
		// Cannot sense beyond this angle
		return
	}

	size := otherData.Radius
	if otherData.Kind == EntityKindDecoy {
		// Decoys appear very large to weapons
		size = 100
	}

	homingStrength := size * baseHomingStrength / (1 + diff.LengthSquared() + 1000*square(square(angleDiff)))
	entity.DirectionTarget = entity.DirectionTarget.Lerp(angle, min(0.95, max(0.01, homingStrength)))
}

// Returns a float in range [0, 1) based on the entity's id.
func (entity *Entity) Hash() float32 {
	const hashSize = 64
	return float32(entity.EntityID&(hashSize-1)) * (1.0 / hashSize)
}

// Returns whether copied turret angles
func (entity *Entity) updateTurretAim(amount Angle) bool {
	turretsCopied := false
	data := entity.Data()
	angles := entity.TurretAngles()

	for i := range angles {
		turretData := data.Turrets[i]
		directionTarget := turretData.Angle
		if target := entity.AimTarget(); target != (Vec2f{}) { // turret target lasts for 5 seconds
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

// Repair regenerates the Entity's health by an amount of Ticks.
func (entity *Entity) Repair(ticks Ticks) {
	if entity.Ticks > ticks {
		entity.Ticks -= ticks
	} else {
		entity.Ticks = 0
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
		return float32(entity.Ticks) / float32(entity.MaxHealth())
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

	weaponData := armamentData.Type.Data()

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
	var angles []Angle
	if entity.Data().Kind == EntityKindBoat {
		angles = entity.TurretAngles()
	}
	return ArmamentTransform(entity.EntityType, entity.Transform, angles, index)
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
				if a.Type == entity.EntityType && consumption[i] == TicksMax {
					consumption[i] = a.Reload()
					return
				}
			}
		}
	}
}

func (entity *Entity) ConsumeArmament(index int) {
	entity.ClearSpawnProtection()

	entity.Owner.ext.copyArmamentConsumption()
	a := &entity.Data().Armaments[index]

	// Limited armaments start their timer when they die.
	reload := TicksMax
	if !a.Type.Data().Limited {
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
	if entity.EntityType == EntityTypeInvalid {
		// Just spawned
		entity.Owner.ext.setSpawnProtection(spawnProtection)
	}

	var oldArmaments []Ticks
	if entity.EntityType != EntityTypeInvalid {
		oldArmaments = entity.ArmamentConsumption()
	}

	ext := &entity.Owner.ext

	entity.EntityType = entityType
	ext.setType(entity.EntityType)

	// Keep similar consumption.
	upgradeArmaments(entityType, entity.ArmamentConsumption(), oldArmaments)
	entity.Ticks /= 2

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
	sensors := entity.Data().Sensors
	visual = sensors.Visual.Range
	radar = sensors.Radar.Range
	sonar = sensors.Sonar.Range
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
