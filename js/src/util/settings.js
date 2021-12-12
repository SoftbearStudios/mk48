// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

import {clamp, mapRanges} from '../util/math.js';
import storage from '../util/storage.js';
import {writable} from 'svelte/store';

const settingStore = function(name, defaultValue, minValue, maxValue) {
	if (typeof defaultValue === 'number') {
		if (minValue == undefined) {
			minValue = 0;
		}
		if (maxValue == undefined) {
			maxValue = 1;
		}
	}

	let initialValue;

	const loadedValue = storage[name];

	if (loadedValue === null || loadedValue === undefined) {
		initialValue = defaultValue;
	} else {
		// Browsers typically store values as strings such as "true"
		if (typeof defaultValue === 'string') {
			initialValue = loadedValue;
		} else {
			try {
				initialValue = JSON.parse(loadedValue);
			} catch (err) {
				console.warn(err);
			}
		}

		if (typeof initialValue != typeof defaultValue) {
			initialValue = defaultValue;
		} else if (typeof initialValue == 'number') {
			initialValue = clamp(initialValue, minValue, maxValue);
		}
	}

	const store = writable(initialValue);
	store.subscribe(newValue => {
		storage[name] = newValue;
	});

	window.addEventListener('storage', () => {
		console.log('storage event');
		if (typeof defaultValue === 'string') {
			store.set(storage[name]);
		} else {
			try {
				store.set(JSON.parse(storage[name]));
			} catch (err) {
				console.warn(err);
			}
		}
	});

	if (typeof defaultValue === 'number') {
		// levels of 5 would mean the possible integers 0, 1, 2, 3, and 4
		store.mapToInteger = (value, levels) => {
			return Math.round(mapRanges(value, minValue, maxValue, 0, levels - 1));
		};
		store.setFromInteger = (value, levels) => {
			store.set(mapRanges(value, 0, levels - 1, minValue, maxValue));
		};
	}
	store.setDefault = () => store.set(defaultValue);
	return store;
};

export const chatOpen = settingStore('chat', true);
export const renderWaves = settingStore('renderWaves', true);
export const renderFoam = settingStore('renderFoam', true);
export const renderTerrainTextures = settingStore('renderTerrainTextures', true);
export const volume = settingStore('volume', 1.0);
export const antialias = settingStore('antialias', true);
export const resolution = settingStore('resolution', 1.0, 0.25, 1.0);
export const fpsCounter = settingStore('fpsCounter', false);