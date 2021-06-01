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

		entityData.speed = Math.min(entityData.speed, 1000);
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
				case 'sam':
				case 'missile':
					maxRange = mapRanges(entityData.length, 1, 10, 500, 1200, true);

					avgSpeed = 0;
					let count = 0;
					let speed = 0;
					const seconds = 0.1;
					for (let d = 0; d < maxRange; d += speed * seconds) {
						let delta = entityData.speed - speed;
						speed += Math.min(delta, 800 * seconds) * seconds;
						avgSpeed += speed;
						count++;
					}

					avgSpeed /= count;
					//console.log(`${entityType}: ${entityData.speed} -> ${avgSpeed}`);
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
			case 'sam':
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
					case 'mine':
						entityData.reload = 30;
						break;
					case 'missile':
					case 'sam':
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
					const copy = {...armament, positionSide: -armament.positionSide, count: undefined};
					if (armament.angle != undefined) {
						copy.angle = -armament.angle;
					}
					entityData.armaments.push(copy);
				}
			}
		}
	}

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

// Sort armaments
for (const entityType of Object.keys(entityDatas)) {
	const entityData = entityDatas[entityType];

	function rankArmament(armament) {
		const armamentEntityData = entityDatas[armament.default];
		// Positive means closer to beginning
		const typeRanks = {
			'torpedo': 10,
			'missile': 9,
			'rocket': 8,
			'shell': ['battleship', 'cruiser'].includes(entityData.subtype) ? 12 : 5,
			'sam': -5,
			'decoy': -8,
			'aircraft': entityData.type == 'carrier' ? 12 : -10,
		}
		return typeRanks[armamentEntityData.subtype] || 0;
	}

	entityData.armaments.sort((first, second) => {
		return rankArmament(second) - rankArmament(first);
	});
}

fs.writeFileSync('./entities.json', JSON.stringify(entityDatas, null, '\t'));
