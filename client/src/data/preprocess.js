// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

import fs from 'fs';
import {mapRanges} from '../util/math.js';

/*
	This file applies basic operations on the raw entity data, such
	as computing a damage value for weapons based on their type and size
	and limiting the range of sensors and weapons to a max value

	This type of data is not dynamic at runtime, but it would be redundant
	to hardcode it into the raw entity data
*/

const entityDatas = JSON.parse(fs.readFileSync('./entities-raw.json'));

for (const entityType of Object.keys(entityDatas)) {
	const entityData = entityDatas[entityType];

	if (entityData.speed) {
		switch (entityData.type) {
			case 'weapon':
				switch (entityData.subtype) {
					case 'shell':
						entityData.speed *= 0.75;
						break;
				}
		}
	}

	if (entityData.range && entityType !== 'depositor') {
		let maxRange = 1500;
		let avgSpeed = entityData.speed
		if (entityData.type === 'weapon') {
			switch (entityData.subtype) {
				case 'shell':
					maxRange = mapRanges(entityData.length, 0.2, 2, 250, 850, true);
					break;
				case 'rocket':
				case 'missile':
					maxRange = mapRanges(entityData.length, 1, 10, 1000, maxRange, true);
					// TODO calculate time taken to go range
					if (entityData.speed > 500) {
						avgSpeed = Math.min(entityData.speed, 200)
					}
					break;
				case 'aircraft':
					maxRange = 5000;
					break;
			}
		}
		entityData.range = Math.min(entityData.range, maxRange);
		let rangeLifespan = Math.max(entityData.range / avgSpeed, 0.1);
		if (!entityData.lifespan || rangeLifespan < entityData.lifespan) {
			entityData.lifespan = rangeLifespan;
		}
		delete(entityData.range);
	}

	if (entityData.type === 'boat') {
		// Anti-aircraft power
		switch (entityData.subtype) {
			case 'dredger':
			case 'submarine':
				break;
			default:
				entityData.antiAircraft = parseFloat(mapRanges(entityData.length, 30, 300, 0.02, 0.25).toFixed(3));
		}

		switch (entityData.subtype) {
			case 'pirate':
				entityData.npc = true;
				break;
		}
	}

	if (entityData.type === 'weapon' && entityData.damage == undefined) {
		switch (entityData.subtype) {
			case 'torpedo':
				entityData.damage = mapRanges(entityData.length, 3, 7, 0.6, 1.1, true);
				// NOTE: This makes homing torpedoes do less damage.
				/*
				if (Array.isArray(entityData.sensors) && entityData.sensors.length > 0) {
					entityData.damage -= 0.1;
				}
				*/
				break;
			case 'mine':
				entityData.damage = 1.5;
				break;
			case 'rocket':
			case 'missile':
				entityData.damage = mapRanges(entityData.length, 1, 6, 0.15, 0.8, true);
				break;
			case 'shell':
				entityData.damage =  mapRanges(entityData.length, 0.25, 2, 0.4, 0.7, true);
				break;
			case 'depthCharge':
				entityData.damage = 0.8;
				break;
		}
	}

	if (entityData.reload == undefined) {
		switch (entityData.type) {
			case 'weapon':
				switch (entityData.subtype) {
					case 'aircraft':
						entityData.reload = 10;
						break;
					case 'depositor':
						entityData.reload = 1;
						break;
					case 'rocket':
						entityData.reload = 2.5;
						break;
					case 'missile':
						entityData.reload = mapRanges(entityData.length, 1, 6, 4, 16, true);
						break;
					case 'shell':
						entityData.reload =  mapRanges(entityData.length, 0.25, 2, 8, 16, true);
						break;
					default:
						entityData.reload = 8;
						break;
				}
				break;
			case 'decoy':
				entityData.reload = 20;
				break;
		}
	}

	const armaments = entityData.armaments;
	entityData.armaments = [];

	const turrets = entityData.turrets;
	entityData.turrets = [];

	if (turrets) {
		for (let i = 0; i < turrets.length; i++) {
			const turret = turrets[i];

			// Degrees to radians
			turret.angle = (turret.angle || 0) * Math.PI / 180;

			const sym = turret.symmetrical;
			delete turret.symmetrical;
			entityData.turrets.push(turret);

			if (sym) {
				const copy = {...turret, angle: -turret.angle, positionSide: -turret.positionSide};
				entityData.turrets.push(copy);
			}
		}

		for (let i = 0; i < entityData.turrets.length; i++) {
			const turret = entityData.turrets[i];

			if (turret.type) {
				for (const armament of entityDatas[turret.type].armaments) {
					armaments.push({...armament, turret: i});
				}
			}
		}
	}

	if (armaments) {
		for (const armament of armaments) {
			if (typeof armament.angle === 'number') {
				armament.angle *= Math.PI / 180;
			}
			const sym = armament.symmetrical;
			delete armament.symmetrical;

			for (let i = 0; i < (armament.count || 1); i++) {
				entityData.armaments.push({...armament, count: undefined});
				if (sym) {
					const copy = {...armament, angle: -armament.angle, positionSide: -armament.positionSide, count: undefined};
					entityData.armaments.push(copy);
				}
			}
		}
	}

	entityData.armaments.sort((first, second) => {
		if (first.airdrop) {
			return 1;
		}
		if (second.airdrop) {
			return -1;
		}
		return 0;
	})

	const sensors = entityData.sensors;
	entityData.sensors = [];

	if (sensors) {
		for (const sensor of sensors) {
			if (typeof sensor.range !== 'number') {
				let base = 0;
				let factor = 0;
				switch (sensor.type) {
					case 'visual':
						base = 500;
						factor = 3;
						break;
					case 'radar':
						base = 1000;
						factor = 2;
						break;
					case 'sonar':
						base = 1000
						factor = 1
						break;
				}

				sensor.range = base + factor * entityData.length;
				sensor.range = Math.min(sensor.range, 2000);
			}
			entityData.sensors.push(sensor);
		}
	}
}

fs.writeFileSync('./entities.json', JSON.stringify(entityDatas, null, '\t'));
