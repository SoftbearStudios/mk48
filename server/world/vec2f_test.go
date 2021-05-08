// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"github.com/chewxy/math32"
	"testing"
)

func approx(a, b float32) bool {
	return math32.Abs(a-b) < 0.0001
}

func TestVec2f_Angle(t *testing.T) {
	tests := []struct {
		vec Vec2f
		ang Angle
	}{
		{Vec2f{0, 0}, 0},
		{Vec2f{1, 1}, Angle(math32.Pi / 4)},
		{Vec2f{0, 1}, Angle(math32.Pi / 2)},
		{Vec2f{0, -1}, Angle(-math32.Pi / 2)},
	}

	for _, test := range tests {
		if !approx(float32(test.ang), float32(test.vec.Angle())) {
			t.Errorf("expected %v -> %f, found %f", test.vec, test.ang, test.vec.Angle())
		}
	}

	for i := Angle(-10); i < 10; i += 0.25 {
		if !approx(0, float32(i.Diff(i.Vec2f().Angle()))) {
			t.Errorf("error, expected %s got %s", i, i.Vec2f().Angle())
		}
	}
}
