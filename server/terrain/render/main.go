// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"flag"
	"fmt"
	"image"
	"image/color"
	"image/png"
	"log"
	terrain "mk48/server/terrain"
	"mk48/server/terrain/compressed"
	"mk48/server/terrain/noise"
	"mk48/server/world"
	"os"
	"runtime/pprof"
)

type ColorVec [3]float32

var colors = [...]ColorVec{
	RGB(0, 50, 115),
	RGB(0, 75, 130),
	RGB(194, 178, 128),
	RGB(90, 180, 30),
	RGB(105, 110, 115),
	Gray(220),
}

func main() {
	var cpuProfile string
	flag.StringVar(&cpuProfile, "cpuprofile", "", "write cpu profile to `file`")
	flag.Parse()

	if cpuProfile != "" {
		f, err := os.Create(cpuProfile)
		if err != nil {
			log.Fatal("could not create CPU profile: ", err)
		}
		defer f.Close() // error handling omitted for example
		if err := pprof.StartCPUProfile(f); err != nil {
			log.Fatal("could not start CPU profile: ", err)
		}
		defer pprof.StopCPUProfile()
	}

	run()
}

func run() {
	t := compressed.New(noise.NewDefault())
	o := float32(-compressed.Size * 0.5 * terrain.Scale)
	s := float32(compressed.Size * terrain.Scale)
	data := t.At(world.AABBFrom(o, o, s, s))

	var buf compressed.Buffer
	buf.Reset(data.Data)
	raw := make([]byte, data.Length)
	_, _ = buf.Read(raw)

	fmt.Printf("compressed to %d%% (%dkb)\n", 100*len(data.Data)/len(raw), len(data.Data)/1024)

	width := data.Stride
	height := data.Length / width
	img := image.NewRGBA(image.Rect(0, 0, width, height))

	for j := 0; j < width; j++ {
		for i := 0; i < height; i++ {
			var c ColorVec

			h := raw[i+j*width]
			switch {
			case h <= terrain.OceanLevel:
				c = colors[0].Lerp(colors[1], clamp(float32(h)/float32(terrain.OceanLevel)))
			case h <= terrain.SandLevel:
				c = colors[2]
			case h <= terrain.GrassLevel:
				c = colors[2].Lerp(colors[3], clamp(float32(h-terrain.SandLevel)*0.05))
			case h <= terrain.RockLevel:
				c = colors[3].Lerp(colors[4], clamp(float32(h-terrain.GrassLevel)*0.1))
			default:
				c = colors[4].Lerp(colors[5], clamp(float32(h-terrain.RockLevel)*0.07))
			}

			img.Set(i, j, c.Color())
		}
	}

	file, err := os.Create("out.png")
	if err != nil {
		log.Fatal(err)
	}
	defer file.Close()

	if err = png.Encode(file, img); err != nil {
		log.Fatal(err)
	}
}

func Gray(v byte) ColorVec {
	return RGB(v, v, v)
}

func RGB(r, g, b byte) ColorVec {
	const factor = 1.0 / 255
	return ColorVec{float32(r) * factor, float32(g) * factor, float32(b) * factor}
}

func (vec ColorVec) String() string {
	return fmt.Sprintf("vec4(%.3f, %.3f, %.3f, 1.0)", vec[0], vec[1], vec[2])
}

func (vec ColorVec) Mul(v float32) ColorVec {
	vec[0] *= v
	vec[1] *= v
	vec[2] *= v
	return vec
}

func (vec ColorVec) Lerp(other ColorVec, factor float32) ColorVec {
	for i := range vec {
		vec[i] = world.Lerp(vec[i], other[i], factor)
	}
	return vec
}

func (vec ColorVec) Color() color.RGBA {
	return color.RGBA{R: floatToByte(vec[0]), G: floatToByte(vec[1]), B: floatToByte(vec[2]), A: 255}
}

func clamp(f float32) float32 {
	if f < 0 {
		return 0
	}
	if f > 1 {
		return 1
	}
	return f
}

func floatToByte(f float32) byte {
	if f < 0 {
		return 0
	}
	if f > 1.0 {
		return 255
	}
	return byte(f * 255)
}
