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
		angles[i] = ToAngle(rand.Float32() * math32.Pi * 2)
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
		angles[i] = ToAngle(rand.Float32() * math32.Pi * 2)
	}
	b.ResetTimer()

	var acc Vec2f
	for i := 0; i < b.N; i++ {
		a := angles[i&(count-1)]
		acc = acc.Add(a.Vec2f())
	}
	_ = acc
}

func TestAngle_EdgeCase(t *testing.T) {
	var angle Angle
	angle.UnmarshalJSON([]byte("-3.141592653589793"))
	float := angle.Float()
	if float > 0 {
		t.Errorf("expected negative pi, found %f", float)
	}

	angle.UnmarshalJSON([]byte("3.141592653589793"))
	float = angle.Float()
	if float < 0 {
		t.Errorf("expected positive pi, found %f", float)
	}
}

func TestAngle_Diff(t *testing.T) {
	errs := 0

	for step := float32(0.01); step < math32.Pi; step += 0.01 {
		for i := -math32.Pi * 2; i < math32.Pi*2; i += step {
			diff := ToAngle(i).Diff(ToAngle(i - step)).Float()
			if !approx(diff, step) {
				if errs++; errs > 20 {
					t.FailNow()
				}
				t.Errorf("%f expected %f, found %f", i, step, diff)
			}
		}

		for i := -math32.Pi * 2; i < math32.Pi*2; i += step {
			diff := ToAngle(i).Diff(ToAngle(i + step)).Float()
			if !approx(diff, -step) {
				if errs++; errs > 20 {
					t.FailNow()
				}
				t.Errorf("%f expected %f, found %f", i, -step, diff)
			}
		}
	}
}
