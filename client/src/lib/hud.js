// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

import entityData from '../data/entities.json';
import {clamp, dist, mapRanges} from '../util/math.js';

export const THROTTLE_START = 0.55;
export const THROTTLE_END = 1;

export function drawHud(hud, entity, sprite, contacts) {
	hud.clear();
	hud.lineStyle(0.5, 0xffffff, 0.25)
	hud.drawCircle(0, 0, sprite.width * THROTTLE_END);
	hud.drawCircle(0, 0, sprite.width * THROTTLE_START);

	const throttle = mapRanges(Math.abs(entity.velocityTarget), 0, entityData[entity.type].speed, THROTTLE_START, THROTTLE_END);
	const speed = mapRanges(Math.abs(entity.velocity), 0, entityData[entity.type].speed, THROTTLE_START, THROTTLE_END);
	hud.lineStyle(0.3, 0xffffff, 0.25)
	hud.drawCircle(0, 0, sprite.width * throttle);
	hud.drawCircle(0, 0, sprite.width * speed);
	if (typeof sprite.directionTarget === 'number') {
		const cos = Math.cos(sprite.directionTarget) * sprite.width;
		const sin = Math.sin(sprite.directionTarget) * sprite.width;
		hud.moveTo(cos * THROTTLE_START, sin * THROTTLE_START);
		hud.lineTo(cos * THROTTLE_END, sin * THROTTLE_END);
	}

	/*
	for (const sensor of entityData[entity.type].sensors) {
		if (sensor.range < 1000) {
			hud.lineStyle(1, 0xff0000, 0.25)
			hud.drawCircle(0, 0, sensor.range);
		}
	}
	*/

	hud.lineStyle(0, 0, 0);

	const scale = sprite.width * THROTTLE_END;

	for (const contact of Object.values(contacts)) {
		const distance = dist(contact.position, sprite.position);

		if (distance < sprite.width * THROTTLE_END) {
			continue;
		}

		const angle = Math.atan2(contact.position.y - sprite.position.y, contact.position.x - sprite.position.x);
		const scaledDistance = scale * mapRanges(distance, 0, 2000, 1.05, 1.5, true);

		let color = 0xee6666;

		if (contact.friendly) {
			color = 0x2ecc71; // green
		} else if (['collectible'].includes(entityData[contact.type].type)) {
			color = 0xf1c40f; // yellow
		} else if (contact.type && ['submarine', 'torpedo'].includes(entityData[contact.type].subtype)) {
			color = 0x3498db; // blue
		}

		hud.beginFill(color, 0.5)
		hud.drawCircle(scaledDistance * Math.cos(angle), scaledDistance * Math.sin(angle), clamp(0.05 * sprite.width, 1, 3) / (contact.uncertainty + 1));
		hud.endFill();
	}
}
