// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

export function getMouseButton(event) {
	let button = 0;

	if ('which' in event) {
		button = event.which - 1;
	} else if ('button' in event) {
		button = event.button;
	}

	return button;
}

export function isMobile() {
	return /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(window.navigator.userAgent);
}

export function hasWebP() {
	const elem = document.createElement('canvas');
	if (elem.getContext && elem.getContext('2d')) {
		return elem.toDataURL('image/webp').indexOf('data:image/webp') == 0;
	}
	return false;
}
