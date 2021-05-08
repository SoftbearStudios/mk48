// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later


export function getInvite() {
	const ref = window.location.href;
	const hashIndex = ref.indexOf('#');
	if (hashIndex > -1) {
		return ref.substring(hashIndex + 1);
	}
	return undefined;
}
