// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

export function angleDiff(angle1, angle2) {
	let angleDifference = (angle2 - angle1) % (Math.PI * 2);
	if (angleDifference < -Math.PI) {
		angleDifference += Math.PI * 2;
	} else if (angleDifference >= Math.PI) {
		angleDifference -= Math.PI * 2;
	}
	return angleDifference;
}

export function clamp(number, min, max) {
	return Math.min(max, Math.max(min, number));
}

export function clampMagnitude(number, max) {
	return clamp(number, -max, max);
}

export function dist(point1, point2) {
	return Math.hypot(point1.x - point2.x, point1.y - point2.y);
}

export function mapRanges(number, oldMin, oldMax, newMin, newMax, clampToRange = false) {
	const oldRange = oldMax - oldMin;
	const newRange = newMax - newMin;
	const numberNormalized = (number - oldMin) / oldRange;
	let mapped = newMin + numberNormalized * newRange;
	if (clampToRange) {
		mapped = clamp(mapped, newMin, newMax);
	}
	return mapped;
}
