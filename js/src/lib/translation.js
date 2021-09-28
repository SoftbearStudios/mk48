// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

import {writable} from 'svelte/store';
import strings from '../data/strings.json';
import storage from '../util/storage.js';

// t is an abbreviation of translation, which is the purpose of this file

function resolve(obj, keys) {
	return keys.split('.').reduce(function (cur, key) {
		return cur ? cur[key] : undefined;
	}, obj);
};

const missings = {};

export function translateAs(lang, key) {
	const translation = resolve(strings[lang], key) || resolve(strings.en, key);
	if (translation) {
		return translation;
	}
	const missing = `missing string for: ${key}`;
	if (!(missing in missings)) {
		console.warn(missing);
		missings[missing] = true;
	}
	return key;
}

const translate = writable(null);

export function setLanguage(lang) {
	if (lang) {
		// Only save if actual choice
		storage.language = lang;
	}
	translate.set(translateAs.bind(null, lang || 'en'));
}

setLanguage(storage.language);

export default translate;
