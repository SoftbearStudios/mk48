// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

export function applyVelocity(position, velocity, direction, seconds) {
	position.x += Math.cos(direction) * seconds * velocity;
	position.y += Math.sin(direction) * seconds * velocity;
}
