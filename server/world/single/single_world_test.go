// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package single

import (
	"mk48/server/world"
	"testing"
)

func BenchmarkSingleWorld(b *testing.B) {
	world.Bench(b, func(_ int) world.World {
		return New()
	}, 4096)
}
