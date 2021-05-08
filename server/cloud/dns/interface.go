// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package dns

import (
	"net"
)

type DNS interface {
	UpdateRoute(region string, slot int, address net.IP) error
}
