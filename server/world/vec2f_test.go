// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"github.com/chewxy/math32"
	"math/rand"
	"testing"
)

func BenchmarkVec2f_Angle(b *testing.B) {
	const count = 1024
	vectors := make([]Vec2f, count)
	for i := range vectors {
		vectors[i] = Vec2f{X: rand.Float32()*100 - 50, Y: rand.Float32()*100 - 50}
	}
	b.ResetTimer()

	var acc Angle
	for i := 0; i < b.N; i++ {
		v := vectors[i&(count-1)]
		acc += v.Angle()
	}
	_ = acc
}

func approx(a, b float32) bool {
	return math32.Abs(a-b) < 0.02
}

func TestVec2f_Angle(t *testing.T) {
	tests := []struct {
		vec Vec2f
		ang Angle
	}{
		{Vec2f{0, 0}, 0},
		{Vec2f{1, 1}, Pi / 4},
		{Vec2f{0, 1}, Pi / 2},
		{Vec2f{0, -1}, Pi / 2 * 3},
	}

	for _, test := range tests {
		if !approx(float32(test.ang), float32(test.vec.Angle())) {
			t.Errorf("expected %v.Angle(): %s, got %s", test.vec, test.ang, test.vec.Angle())
		}
	}

	for i := float32(-10.0); i < 10; i += 0.25 {
		a := ToAngle(i)
		a2 := a.Vec2f().Angle()
		if !approx(0, a.Diff(a2).Float()) {
			t.Errorf("expected %s got %s", a, a2)
		}
	}
}
