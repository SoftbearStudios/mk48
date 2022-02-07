// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

import {clamp, mapRanges} from '../util/math.js';
import storage from '../util/storage.js';
import {get, writable} from 'svelte/store';

const rustSettingStores = [];

export const loadRustSettings = function() {
	for (const s of rustSettingStores) {
		s()
	}
}

const rustSettingStore = function(name) {
	const store = {inner: writable(undefined)};
	store.subscribe = function(c) {
		function load() {
			this.inner.set(window.rust.getSetting(name));
		}
		let l = load.bind(this);

		if (window.rust && get(this.inner) === undefined) {
			l()
		} else {
			rustSettingStores.push(l)
		}
		return this.inner.subscribe(c);
	}
	store.inner.subscribe(newValue => {
		window.rust && window.rust.setSetting(name, newValue);
	});
	store.mapToInteger = (value, levels) => {
		return Math.round(mapRanges(value, 0, 1, 0, levels - 1));
	};
	store.setFromInteger = (value, levels) => {
		store.inner.set(mapRanges(value, 0, levels - 1, 0, 1));
	};
	store.set = value => store.inner.set(value);
	return store;
}

const jsSettingsStore = function(name, defaultValue, minValue, maxValue) {
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

export const chatShown = jsSettingsStore('chatShown', true);
export const fpsShown = jsSettingsStore('fpsShown', false);
export const leaderboardShown = jsSettingsStore('leaderboardShown', true);
export const shipControlsShown = jsSettingsStore('shipControlsShown', true);
export const teamsShown = jsSettingsStore('teamsShown', true);
export const upgradeShown = jsSettingsStore('upgradeShown', true);
export const resolution = jsSettingsStore('resolution', 1.0, 0.25, 1.0);

export const waveQuality = rustSettingStore( 'waveQuality');
export const animations = rustSettingStore( 'animations');
export const volume = rustSettingStore( 'volume');
export const antialias = rustSettingStore( 'antialias');

// Not persisted, because that might trap users in cinematic mode.
export const cinematic = writable(false);
