// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

import (
	"github.com/SoftbearStudios/mk48/server/world"
)

// Team is an extension of world.Team with extra data
type Team struct {
	world.Team
	Chats []Chat
}
