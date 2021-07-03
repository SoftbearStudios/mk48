// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"fmt"
	"github.com/SoftbearStudios/mk48/server/world"
	"github.com/nfnt/resize"
	"image"
	"image/png"
	"os"
)

func main() {
	for entityType := world.EntityType(1); entityType < world.EntityType(world.EntityTypeCount); entityType++ {
		file, err := os.Open("../client/static/entities/" + entityType.String() + ".png")
		if err != nil {
			panic(err)
		}

		i, err := png.Decode(file)
		if err != nil {
			panic(err)
		}
		img := i.(*image.NRGBA).SubImage(i.Bounds().Inset(1))

		bounds := img.Bounds()
		width := bounds.Dx()
		height := bounds.Dy()
		aspect := float32(width) / float32(height)

		data := entityType.Data()
		newWidth := metersToPixels(data.Length)
		newHeight := metersToPixels(data.Width)
		newAspect := float32(newWidth) / float32(newHeight)

		switch {
		case newAspect < aspect-0.001:
			newHeight = int(newAspect / aspect * float32(newHeight))
		case newAspect > aspect+0.001:
			newWidth = int(aspect / newAspect * float32(newWidth))
		}

		newImg := resize.Resize(uint(newWidth), uint(newHeight), img, resize.Lanczos3)

		outputFile, err := os.Create("entities_out/" + entityType.String() + ".png")
		if err != nil {
			panic(err)
		}

		if err = png.Encode(outputFile, newImg); err != nil {
			panic(err)
		}

		fmt.Printf("%s %dx%d -> %dx%d\n", entityType.String(), width, height, newWidth, newHeight)
	}
}

func metersToPixels(meters float32) int {
	//meters = math32.Sqrt(math32.Pow(5 * meters, 2)+math32.Pow(64, 2))
	const m = 8
	meters = (1024.0-m)/(1024.0/5)*meters + m
	if meters > 1024 {
		return 1024
	}
	return int(meters)
}
