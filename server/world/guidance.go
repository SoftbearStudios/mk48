// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

type Guidance struct {
	DirectionTarget Angle   `json:"directionTarget,omitempty"`
	VelocityTarget  float32 `json:"velocityTarget,omitempty"`
}
