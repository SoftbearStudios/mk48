// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

import {writable} from 'svelte/store';
import strings from '../data/strings.json';
import storage from '../util/storage.js';

// t is an abbreviation of translation, which is the purpose of this file

const debug = false;
const missings = {};
const translation = writable(null);

setLanguage(storage.language);

export function getLanguage() {
	return storage.language || 'en';
}

export function getLanguageList() {
	let list = [];
	for (let k of Object.keys(strings)) {
		if (!debug && k.startsWith("xx-")) {
			continue;
		}
		if (Object.keys(strings[k]).length > 0) {
			list.push(k);
        }
	}
	return list;
}

function resolve(obj, keys) {
	return keys.split('.').reduce(function (cur, key) {
		return cur ? cur[key] : undefined;
	}, obj);
};

export function setLanguage(lang) {
	if (lang) {
		// Only save if actual choice
		storage.language = lang;
	}
	translation.set(translate.bind(null, lang || 'en'));
}

export function translate(lang, key) {
	const t = resolve(strings[lang], key) || resolve(strings.en, key);
	if (t) {
		return t;
	}
	const missing = `missing string for: ${key}`;
	if (!(missing in missings)) {
		console.warn(missing);
		missings[missing] = true;
	}
	return key;
}

export default translation;
