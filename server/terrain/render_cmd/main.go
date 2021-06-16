// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"flag"
	"fmt"
	terrain "github.com/SoftbearStudios/mk48/server/terrain"
	"github.com/SoftbearStudios/mk48/server/terrain/noise"
	"image/png"
	"log"
	"os"
	"runtime/pprof"
)

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
	for i := int64(50); i < 200; i++ {
		t := terrain.New(noise.New(i, 0, 0))
		img := terrain.Render(t, terrain.Size)

		file, err := os.Create(fmt.Sprintf("out-%d.png", i))
		if err != nil {
			log.Fatal(err)
		}
		defer file.Close()

		if err = png.Encode(file, img); err != nil {
			log.Fatal(err)
		}
	}
}
