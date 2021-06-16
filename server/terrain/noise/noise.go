// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package noise

import (
	"github.com/SoftbearStudios/mk48/server/terrain"
	"github.com/SoftbearStudios/mk48/server/world"
	"github.com/aquilax/go-perlin"
)

const (
	frequency     = 0.001
	zoneFrequency = 0.00015
)

// Generator generates a heightmap using perlin noise.
type Generator struct {
	// Land/coast heightmap noise
	landHi *perlin.Perlin // for smaller/higher frequency details
	landLo *perlin.Perlin // for larger/lower frequency details

	// Open water depth floor heightmap noise
	waterLo *perlin.Perlin

	offset world.Vec2f
}

func NewDefault() *Generator {
	return New(terrain.Seed, terrain.OffsetX, terrain.OffsetY)
}

// New creates a new Generator with a seed.
func New(seed int64, offsetX, offsetY float32) *Generator {
	return &Generator{
		landHi:  perlin.NewPerlin(1.5, 2.0, 4, seed),
		landLo:  perlin.NewPerlin(2.5, 3.0, 4, seed+1),
		waterLo: perlin.NewPerlin(2, 3.0, 3, seed+2),
		offset:  world.Vec2f{X: offsetX, Y: offsetY}.Mul(1.0 / terrain.Scale), // Scale to terrain space
	}
}

// Generate implements terrain.Source.Generate.
func (g *Generator) Generate(px, py, width, height int) []byte {
	buf := make([]byte, width*height)

	// Offsets in terrain space
	offX := float64(g.offset.X) + float64(px)
	offY := float64(g.offset.Y) + float64(py)

	for j := 0; j < height; j++ {
		for i := 0; i < height; i++ {
			x := (float64(i) + offX) * terrain.Scale
			y := (float64(j) + offY) * terrain.Scale

			h := g.landHi.Noise2D(x*frequency, y*frequency)*250 + terrain.SandLevel - 50

			// Zone is very low frequency
			zone := g.landLo.Noise2D(x*zoneFrequency, y*zoneFrequency)*2.0 + 0.4
			if zone > 1 {
				zone = 1
			}
			h *= zone

			depthFloor := clamp((g.waterLo.Noise2D(x*zoneFrequency, y*zoneFrequency)+0.3)*4, 0, 1) * terrain.SandLevel

			buf[i+j*width] = clampToByte(max(h, depthFloor))
		}
	}

	return buf
}
