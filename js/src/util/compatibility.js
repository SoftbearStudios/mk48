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

let cacheHasWebP = null;

function hasWebPSlow() {
	const canvas = document.createElement('canvas');
	// No need to allocate large amounts of memory.
	canvas.width = 32;
	canvas.height = 32;
	if (canvas.getContext && canvas.getContext('2d')) {
		return canvas.toDataURL('image/webp').indexOf('data:image/webp') == 0;
	}
	return false;
}

export function hasWebP() {
	if (cacheHasWebP === null) {
		cacheHasWebP = hasWebPSlow();
	}
	return cacheHasWebP;
}
