// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package tree

import (
	"mk48/server/world"
	"testing"
)

func BenchmarkTreeWorld(b *testing.B) {
	world.Bench(b, func(radius int) world.World {
		return New(radius)
	}, 4096)
}
