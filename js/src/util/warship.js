// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

import entityData from '../data/entities.json';
import {clamp} from '../util/math.js';

let boatTypeCount = 0;
let boatLevelMax = 0;
let weaponSubTypeCount = 0;
const weaponSubKinds = {};
for (const entityType of Object.keys(entityData)) {
    const data = entityData[entityType];
    switch (data.kind) {
        case 'boat':
            boatTypeCount++;
            boatLevelMax = Math.max(boatLevelMax, data.level);
            break;
        case 'aircraft':
        case 'decoy':
        case 'weapon':
            weaponSubKinds[data.subkind] = true;
            break;
    }
}

export const BOAT_TYPE_COUNT = boatTypeCount;
export const BOAT_LEVEL_MAX = boatLevelMax;
export const WEAPON_SUB_KIND_COUNT = Object.keys(weaponSubKinds).length;

function reloaded(reloads, index) {
	return (!reloads || reloads.length <= index) ? 0 : reloads[index]
}

export function availableShips(level, excludedType) {
	const list = [];
	for (const entityType of Object.keys(entityData)) {
		if (excludedType && entityType == excludedType) {
			continue;
		}
		const data = entityData[entityType];
		if (data.kind === 'boat' && data.level === level && !data.npc) {
			list.push(entityType);
		}
	}
	return list;
}

export function canUpgrade(type, score) {
	return progressOfUpgrade(type, score) === 1 && hasUpgrades(type);
}

export function getArmamentType(armamentData) {
	const aED = entityData[armamentData.type];
	return `${aED.kind}/${aED.subkind}`;
}

export function groupArmaments(armaments, consumptions) {
	const groups = {};
	for (let i = 0; i < armaments.length; i++) {
		const armament = armaments[i];

		const type = getArmamentType(armament);

		let group = groups[type];
		if (!group) {
			group = {type: armament.type, ready: 0, total: 0};
			groups[type] = group;
		}
		group.total++;

		if (reloaded(consumptions, i)) {
			group.ready++;
		}
	}

	return Object.entries(groups);
}

export function hasArmament(consumption, index) {
	return reloaded(consumption, index)
}

export function hasUpgrades(type) {
	return availableShips(entityData[type].level + 1).length > 0;
}

export function levelToScore(level) {
	// Must match rust code
	return (level * level + Math.pow(2, Math.max(level - 3, 0)) - 2) * 10;
}

export function nextLevel(type) {
	return entityData[type].level + 1;
}

export function progressOfUpgrade(type, score) {
	const level = entityData[type].level;
	return clamp(((score || 0) - levelToScore(level)) / (levelToScore(level + 1) - levelToScore(level)), 0, 1);
}

export function summarizeType(translation, type) {
	const data = entityData[type];
	return translation(`kind.${data.kind}.${data.subkind}.name`);
}

export function scoreToLevel(score) {
	let level = 1;
	const governor = 99;
	for (let i = 1; i < governor; i++) {
		if (levelToScore(i) > score) {
			break;
        }
		level = i;
    }
	return level;
}

export function toKnotsString(speed) {
	return `${((speed || 0) * 1.943844492).toFixed(1)}kn`;
}
