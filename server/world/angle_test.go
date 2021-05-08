// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"github.com/chewxy/math32"
	"math/rand"
	"testing"
)

func BenchmarkAngle_Diff(b *testing.B) {
	const count = 1024
	angles := make([]Angle, count)
	for i := range angles {
		angles[i] = Angle(rand.Float32() * math32.Pi * 2)
	}
	b.ResetTimer()

	var acc Angle
	for i := 0; i < b.N; i++ {
		a := angles[i&(count-1)]
		b := angles[(i+count/2)&(count-1)]
		acc += a.Diff(b)
	}
	_ = acc
}

func BenchmarkAngle_Vec2f(b *testing.B) {
	const count = 1024
	angles := make([]Angle, count)
	for i := range angles {
		angles[i] = Angle(rand.Float32() * math32.Pi * 2)
	}
	b.ResetTimer()

	var acc Vec2f
	for i := 0; i < b.N; i++ {
		a := angles[i&(count-1)]
		acc = acc.Add(a.Vec2f())
	}
	_ = acc
}

func TestAngle_Diff(t *testing.T) {
	for step := Angle(0.01); step < Angle(math32.Pi); step += 0.01 {
		for i := Angle(-math32.Pi * 2); i < Angle(math32.Pi*2); i += step {
			if !approx(float32(i.Diff(i-step)), float32(step)) {
				t.Errorf("%f expected %f, found %f", i, step, i.Diff(i-step))
			}
		}

		for i := Angle(-math32.Pi * 2); i < Angle(math32.Pi*2); i += step {
			if !approx(float32(i.Diff(i+step)), float32(-step)) {
				t.Errorf("%f expected %f, found %f", i, -step, i.Diff(i+step))
			}
		}
	}
}
