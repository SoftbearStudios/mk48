// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

// Vertically packed, square image spritesheets only
export function linearSpritesheet(PIXI, baseTexture, resolution, count) {
	const textures = [];

	const orig = new PIXI.Rectangle(0, 0, resolution, count * resolution);
	for (let i = 0; i < count; i++) {
		const rect = new PIXI.Rectangle(0, i * resolution, resolution, resolution);
		textures.push(new PIXI.Texture(baseTexture, rect, orig));
	}

	return textures;
}
