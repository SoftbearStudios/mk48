// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"github.com/chewxy/math32"
)

// Collider is anything that can collide with an Entity but can't be collided back such as terrain.
// It also is required to not change while inside the world radius.
type Collider interface {
	Collides(entity *Entity, seconds float32) bool
}

// Entities within this of eachother's altitudes can collide
const AltitudeCollisionThreshold = 0.25

// Returns if entities have sufficiently similar altitudes to collide
func (entity *Entity) AltitudeOverlap(other *Entity) bool {
	entityData, otherData := entity.Data(), other.Data()
	var boat, weapon *Entity

	if entityData.Kind == EntityKindBoat {
		boat = entity
	} else if otherData.Kind == EntityKindBoat {
		boat = other
	}

	if entityData.Kind == EntityKindWeapon {
		weapon = entity
	} else if otherData.Kind == EntityKindWeapon {
		weapon = other
	}

	if boat != nil && weapon != nil && boat.Altitude() <= 0.0 {
		subKind := weapon.Data().SubKind
		if subKind == EntitySubKindDepthCharge || subKind == EntitySubKindTorpedo || subKind == EntitySubKindMine {
			// Until depth change for weapons is modeled:
			// - Depth charges/mines can hit submerged submarines regardless of depth
			// - All torpedoes hit submerged submarines
			return true
		}
	}

	return math32.Abs(entity.Altitude()-other.Altitude()) <= AltitudeCollisionThreshold
}

// Collides does a rectangle to rectangle collision with another Entity.
// Does not take into account altitude
func (entity *Entity) Collides(otherEntity *Entity, seconds float32) bool {
	data := entity.Data()
	otherData := otherEntity.Data()

	sweep := seconds * entity.Velocity.Float()
	otherSweep := seconds * otherEntity.Velocity.Float()

	r2 := data.Radius + otherData.Radius + sweep + otherSweep
	r2 *= r2

	// More precise version would offset the positions by sweep / 2 but would require a sqrt to calculate new radius
	if entity.Position.DistanceSquared(otherEntity.Position) > r2 {
		return false
	}

	// SAMs collide if within radius, simulating their blast-fragmentation warheads
	if data.SubKind == EntitySubKindSAM || otherData.SubKind == EntitySubKindSAM {
		return true
	}

	dimensions := Vec2f{X: data.Length + sweep, Y: data.Width}
	otherDimensions := Vec2f{X: otherData.Length + otherSweep, Y: otherData.Width}

	normal := entity.Direction.Vec2f()
	otherNormal := otherEntity.Direction.Vec2f()

	return satCollision(entity.Position.AddScaled(normal, sweep*0.5), otherEntity.Position, normal, otherNormal, dimensions, otherDimensions) &&
		satCollision(otherEntity.Position.AddScaled(otherNormal, otherSweep*0.5), entity.Position, otherNormal, normal, otherDimensions, dimensions)
}

// Rectangle-based separating axis theorem collision
func satCollision(position, otherPosition, axisNormal, otherAxisNormal, dimensions, otherDimensions Vec2f) bool {
	// Dimensions
	otherDimensions = otherDimensions.Mul(0.5)
	dimensions = dimensions.Mul(0.5)
	otherAxisTangent := otherAxisNormal.Rot90()

	// Normal vectors scaled to dimensions
	otherScaledNormal := otherAxisNormal.Mul(otherDimensions.X)
	otherScaledTangent := otherAxisTangent.Mul(otherDimensions.Y)

	// All corner positions of other
	otherPosition1 := otherPosition.Add(otherScaledNormal)
	otherPosition2 := otherPosition1.Sub(otherScaledTangent)
	otherPosition1 = otherPosition1.Add(otherScaledTangent)

	otherPosition3 := otherPosition.Sub(otherScaledNormal)
	otherPosition4 := otherPosition3.Add(otherScaledTangent)
	otherPosition3 = otherPosition3.Sub(otherScaledTangent)

	for f := 0; f < 4; f++ {
		// Current dimension
		dimension := dimensions.X
		if f&1 == 1 {
			dimension = dimensions.Y
		}

		// Faster than multiple dot products
		dot := position.Dot(axisNormal)

		// dimension is always positive so minimum must be less than maximum
		minimum := dot - dimension
		maximum := dot + dimension

		// Unrolled loop ~70ns to ~60ns
		d := otherPosition1.Dot(axisNormal)
		otherMin := d
		otherMax := d

		d = otherPosition2.Dot(axisNormal)
		otherMin = min(otherMin, d)
		otherMax = max(otherMax, d)

		d = otherPosition3.Dot(axisNormal)
		otherMin = min(otherMin, d)
		otherMax = max(otherMax, d)

		d = otherPosition4.Dot(axisNormal)
		otherMin = min(otherMin, d)
		otherMax = max(otherMax, d)

		// Not colliding
		if minimum > otherMax || otherMin > maximum {
			return false
		}

		// Faster rotation
		axisNormal = axisNormal.Rot90()
	}

	return true
}
