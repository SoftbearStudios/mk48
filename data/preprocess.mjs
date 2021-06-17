// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

import fs from 'fs';
import {mapRanges} from '../client/src/util/math.js';

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
		switch (entityData.kind) {
			case 'weapon':
				switch (entityData.subkind) {
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
		switch (entityData.kind) {
			case 'weapon':
				switch (entityData.subkind) {
					case 'shell':
						maxRange = mapRanges(entityData.length, 0.2, 2, 250, 850, true);
						break;
					case 'sam':
						maxRange *= 0.5;
					case 'rocket':
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
				}
				break;
			case 'aircraft':
				maxRange = 5000;
				break;
		}
		entityData.range = Math.min(entityData.range, maxRange);
		let rangeLifespan = Math.max(entityData.range / avgSpeed, 0.1);
		if (!entityData.lifespan || rangeLifespan < entityData.lifespan) {
			entityData.lifespan = rangeLifespan;
		}
		delete(entityData.range);
	}

	if (entityData.kind === 'boat') {
		// Anti-aircraft power
		switch (entityData.subkind) {
			case 'dredger':
			case 'submarine':
				break;
			default:
				entityData.antiAircraft = parseFloat(mapRanges(entityData.length, 30, 300, 0.02, 0.25).toFixed(3));
		}

		switch (entityData.subkind) {
			case 'pirate':
				entityData.npc = true;
				break;
		}
	}

	if (entityData.damage == undefined) {
		switch (entityData.kind) {
			case 'boat':
				// Damage means health (i.e. how much damage before death)
				const factor = 20 / 10 / 60;
				entityData.damage = Math.max(factor, factor * entityData.length);
				break;
			case 'weapon':
				// Damage means damage dealt
				switch (entityData.subkind) {
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
				break;
		}
	}

	if (entityData.reload == undefined) {
		switch (entityData.kind) {
			case 'weapon':
				switch (entityData.subkind) {
					case 'depositor':
						entityData.reload = 1;
						break;
					case 'rocket':
						if (entityData.armaments && entityData.armaments.length > 0) {
							// ASROC
							entityData.reload = 20;
						} else {
							entityData.reload = 2.5;
						}
						break;
					case 'mine':
						entityData.reload = 30;
						break;
					case 'sam':
						entityData.reload = 16;
						break;
					case 'missile':
						entityData.reload = mapRanges(entityData.length, 1, 6, 4, 16, true);
						break;
					case 'shell':
						entityData.reload =  mapRanges(entityData.length, 0.25, 2, 8, 16, true);
						break;
					case 'torpedo':
						entityData.reload = 8;
						if (entityData.sensors && Object.keys(entityData.sensors).length > 0) {
							// Homing torpedoes take longer to reload
							entityData.reload *= 1.5;
						}
						break;
					default:
						entityData.reload = 8;
						break;
				}
				break;
			case 'aircraft':
				entityData.reload = 10;
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

			// Apply azimuth abbreviations
			if (turret.azimuth != undefined) {
				turret.azimuthB = turret.azimuth;
				turret.azimuthF = turret.azimuth;
				delete turret.azimuth;
			}
			if (turret.azimuthF != undefined) {
				turret.azimuthFL = turret.azimuthF;
				turret.azimuthFR = turret.azimuthF;
				delete turret.azimuthF;
			}
			if (turret.azimuthB != undefined) {
				turret.azimuthBL = turret.azimuthB;
				turret.azimuthBR = turret.azimuthB;
				delete turret.azimuthB;
			}

			// Degrees to radians
			if (turret.azimuthFL != undefined) {
				turret.azimuthFL *= Math.PI / 180;
			}
			if (turret.azimuthFR != undefined) {
				turret.azimuthFR *= Math.PI / 180;
			}
			if (turret.azimuthBL != undefined) {
				turret.azimuthBL *= Math.PI / 180;
			}
			if (turret.azimuthBR != undefined) {
				turret.azimuthBR *= Math.PI / 180;
			}

			const sym = turret.symmetrical;
			delete turret.symmetrical;
			entityData.turrets.push(turret);

			if (sym) {
				const copy = {...turret, angle: -turret.angle, azimuthFL: turret.azimuthFR, azimuthFR: turret.azimuthFL, azimuthBL: turret.azimuthBR, azimuthBR: turret.azimuthBL, positionSide: -turret.positionSide};
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

	if (entityData.sensors) {
		for (const sensorType in entityData.sensors) {
			const sensor = entityData.sensors[sensorType];

			if (typeof sensor.range !== 'number') {
				let base = 0;
				let factor = 0;
				switch (sensorType) {
					case 'visual':
						base = 500;
						factor = 2.5;
						break;
					case 'radar':
						base = 1000;
						factor = 1.5;
						break;
					case 'sonar':
						base = 350
						factor = 0.5
						break;
				}

				sensor.range = base + factor * entityData.length;
				sensor.range = Math.min(sensor.range, 2000);
			}
		}
	}
}

// Sort armaments
for (const entityType of Object.keys(entityDatas)) {
	const entityData = entityDatas[entityType];

	function rankArmament(armament) {
		const armamentEntityData = entityDatas[armament.type];
		if (armamentEntityData.kind === 'decoy') {
			return -8;
		}
		// Positive means closer to beginning
		const kindRanks = {
			'weapon/torpedo': 10,
			'weapon/missile': 9,
			'weapon/rocket': 8,
			'weapon/shell': ['battleship', 'cruiser'].includes(entityData.subkind) ? 12 : 5,
			'weapon/sam': -5,
			'decoy/': -8,
			'aircraft/': entityData.subkind == 'carrier' ? 12 : -10,
		}
		const armamentKind = `${armamentEntityData.kind}/${armamentEntityData.subkind}`;
		for (const kind in kindRanks) {
			if (armamentKind.startsWith(kind)) {
				// Match
				return kindRanks[kind];
			}
		}
		return 0;
	}

	entityData.armaments.sort((first, second) => {
		return rankArmament(second) - rankArmament(first);
	});
}

fs.writeFileSync('../server/world/entities.json', JSON.stringify(entityDatas, null, '\t'));
