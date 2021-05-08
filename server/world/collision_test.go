// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"fmt"
	"github.com/chewxy/math32"
	"math/rand"
	"testing"
)

func TestEntity_Collides(t *testing.T) {
	colliding := collidingEntities()
	if !colliding[0].Collides(&colliding[1], 0.1) {
		t.Errorf("expected collision")
	}

	nonColliding := nonCollidingEntities()
	if nonColliding[0].Collides(&nonColliding[1], 0.1) {
		t.Errorf("expected no collision")
	}
}

func BenchmarkEntity_Collides(b *testing.B) {
	for i := 4; i <= 8; i++ {
		radius := float32(int(1) << i)
		b.Run(fmt.Sprintf("Radius%.0f", radius), func(b *testing.B) {
			const count = 1024
			entities := make([]Entity, count)
			for i := range entities {
				entities[i] = randomEntity(radius)
			}
			b.ResetTimer()

			for i := 0; i < b.N; i++ {
				a := &entities[i&(count-1)]
				b := &entities[(i+count/2)&(count-1)]
				_ = a.Collides(b, 0.1)
			}
		})
	}
}

func collidingEntities() [2]Entity {
	return [...]Entity{
		{
			Transform:  Transform{Position: Vec2f{X: 5.0, Y: 5.0}, Direction: Angle(math32.Pi / 4)},
			EntityType: ParseEntityType("fairmileD"),
		},
		{
			Transform:  Transform{Position: Vec2f{X: -5.0, Y: 5.0}, Direction: Angle(math32.Pi * 3 / 5)},
			EntityType: ParseEntityType("komar"),
		},
	}
}

func nonCollidingEntities() [2]Entity {
	return [...]Entity{
		{
			Transform:  Transform{Position: Vec2f{X: 10.0, Y: 10.0}, Direction: Angle(math32.Pi / 4)},
			EntityType: ParseEntityType("fairmileD"),
		},
		{
			Transform:  Transform{Position: Vec2f{X: -10.0, Y: 5.0}, Direction: Angle(math32.Pi * 3 / 5)},
			EntityType: ParseEntityType("komar"),
		},
	}
}

func randomEntity(radius float32) Entity {
	return Entity{
		Transform:  Transform{Position: Vec2f{X: rand.Float32()*radius*2 - radius, Y: rand.Float32()*radius*2 - radius}},
		EntityType: EntityType(rand.Intn(EntityTypeCount-1) + 1),
	}
}
