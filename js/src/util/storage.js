// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

const fakeStorage = {};

const proxy = new Proxy({}, {
	get: function(obj, key) {
		try {
			return localStorage[key];
		} catch (err) {
			return fakeStorage[key];
		}
	},
	set: function(obj, key, value) {
		try {
			localStorage[key] = value;
		} catch(err) {

		} finally {
			fakeStorage[key] = '' + value;
		}
		return true;
	},
	deleteProperty: function(obj, key) {
		try {
			delete localStorage[key];
		} catch(err) {

		} finally {
			delete fakeStorage[key];
		}
		return true;
	}
});

export default proxy;
